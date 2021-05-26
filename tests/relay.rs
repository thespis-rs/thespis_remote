// Tests:
//
// ✔ Basic relaying
// ✔ relay multi deep
// ✔ Test unknown service error in a relay
// ✔ Relay dissapeared event
// ✔ try to send after relay has dissapeared (should give mailboxclosed)
// ✔ relay call error from relay
// ✔ propagating errors over several relays deep -> TODO, see test, some errors are in non determined order


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// Test relaying messages
//
#[async_std::test]
//
async fn relay_once()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );

	let provider = async move
	{
		// Create mailbox for our handler
		//
		debug!( "start mailbox for handler" );
		let addr_handler = Addr::builder().start( Sum(0), &AsyncStd ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( addr_handler.clone_box() );
		sm.register_handler::<Show>( addr_handler.clone_box() );


		// get a framed connection
		//
		debug!( "start mailbox for provider" );
		let (peer_addr, _peer_evts, handle) = peer_listen( ab, Arc::new( sm ), AsyncStd, "provider" ).await;

		drop( peer_addr );

		handle.await;

		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		debug!( "start mailbox for consumer_to_relay" );

		let (mut to_relay, _)  = peer_connect( cb, AsyncStd, "consumer_to_relay" ).await;

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_relay.clone() );

		assert_eq!( Ok(()), addr.call( Add(5) ).await );
		assert_eq!( Ok(()), addr.call( Add(5) ).await );

		let resp = addr.call( Show ).await.expect( "Call failed" );

		assert_eq!( 10, resp );

		warn!( "consumer end, telling relay to close connection" );

		to_relay.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );

		warn!( "consumer end, relay processed CloseConnection" );
	};

	let relays = async move
	{
		relay( ba, bc, Box::pin( consumer ), true, AsyncStd ).await;

		warn!( "relays end" );
	};

	join( provider, relays ).await;
}



// Test relaying several relays deep
//
#[async_std::test]
//
async fn relay_multi()
{
	// flexi_logger::Logger::with_str( "relay=trace, thespis_impl=info, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );
	let (cd, dc) = Endpoint::pair( 64, 64 );
	let (de, ed) = Endpoint::pair( 64, 64 );
	let (ef, fe) = Endpoint::pair( 64, 64 );

	let provider = async move
	{
		// get a framed connection
		//
		let (_, _, handle) = peer_listen( ab, Arc::new( add_show_sum() ), AsyncStd, "provider" ).await;

		handle.await;

		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		let (mut relay, _)  = peer_connect( fe, AsyncStd, "consumer_to_relay" ).await;

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( relay.clone() );

		addr.call( Add(5) ).await.expect( "Send failed" );

		assert_eq!( Ok(()), addr.call( Add(5) ).await );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 10, resp );

		relay.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to nodeb" );
	};

	let relays = async move
	{
		let  relay4 = relay( ed, ef, Box::pin( consumer ), true, AsyncStd )       ;
		let  relay3 = relay( dc, de, Box::pin( relay4   ), true, AsyncStd )       ;
		let  relay2 = relay( cb, cd, Box::pin( relay3   ), true, AsyncStd )       ;
		              relay( ba, bc, Box::pin( relay2   ), true, AsyncStd ).await ;
	};

	join( provider, relays ).await;
}



// Test unknown service error in a relay
//
#[async_std::test]
//
async fn relay_unknown_service()
{
	// flexi_logger::Logger::with_str( "relay=trace, thespis_impl=info, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );

	let provider = async move
	{
		// get a framed connection
		//
		let (_, _, handle) = peer_listen( ab, Arc::new( add_show_sum() ), AsyncStd, "provider" ).await;

		handle.await;

		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		let (mut relay, _relay_evts) = peer_connect( cb, AsyncStd, "consumer_to_relay" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid  = ServiceID::from( 1 );

		let mut wf = ThesWF::with_capacity( std::mem::size_of::<Add>() );
		wf.set_sid( sid );
		serde_cbor::to_writer( &mut wf, &Add(5) ).unwrap();

		let call = Call::new( wf );

		let rx = relay.call( call ).await

			.expect( "call peer" )
			.expect( "send out ms" )
		;

		assert!(matches!
		(
			rx.await.expect( "return error, don't drop connection" ).unwrap_err(),
			ConnectionError::UnknownService{ sid, .. } if sid == sid.into()
		));


		relay.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to nodeb" );
	};


	let relay = relay( ba, bc, Box::pin( consumer ), true, AsyncStd );

	let provi_handle = AsyncStd.spawn_handle( provider ).expect( "Spawn provider"  );
	let relay_handle = AsyncStd.spawn_handle( relay    ).expect( "Spawn relays"  );

	join( provi_handle, relay_handle ).await;
}
