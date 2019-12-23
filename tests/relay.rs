// Tests:
//
// - ✔ Basic relaying
// - ✔ relay multi deep
// - ✔ Test unknown service error in a relay
// - ✔ Relay dissapeared event
// - ✔ try to send after relay has dissapeared (should give mailboxclosed)
// - ✔ relay call error from relay
// - ✔ propagating errors over several relays deep -> TODO, see test, some errors are in non determined order


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// Helper method to create relays
//
async fn relay
(
	connect   : Endpoint,
	listen    : Endpoint,
	next      : Pin<Box< dyn Future<Output=()> + Send >>,
	relay_show: bool,
	exec      : impl Spawn
)
{
	debug!( "start mailbox for relay_to_provider" );

	let (mut provider_addr, provider_evts) = peer_connect( connect, &exec, "relay_to_provider" ).await;
	let provider_addr2                     = provider_addr.clone();

	// Relay part ---------------------

	let relay = async move
	{
		let (srv_sink, srv_stream) = connect_return_stream( listen ).await;

		// Create mailbox for peer
		//
		let mb_peer  : Inbox<Peer<MS>> = Inbox::new( "relay_to_consumer".into() );
		let peer_addr                  = Addr ::new( mb_peer.sender() );

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::new( peer_addr, srv_stream, srv_sink ).expect( "spawn peer" );

		let add  = <Add   as Service<remotes::Services>>::sid();
		let show = <Show  as Service<remotes::Services>>::sid();

		let relayed = if relay_show
		{
			vec![ add, show ]
		}

		else { vec![ add ] };

		peer.register_relayed_services( relayed, provider_addr2, provider_evts ).expect( "register relayed" );

		debug!( "start mailbox for relay_to_consumer" );
		mb_peer.start_fut( peer ).await;
		warn!( "relay async block finished" );
	};


	let (relay_fut, relay_outcome) = relay.remote_handle();
	exec.spawn( relay_fut ).expect( "failed to spawn server" );

	// we need to spawn this after peerb, otherwise peerb is not listening yet when we try to connect.
	//
	exec.spawn( next ).expect( "Spawn next" );


	// If the nodec closes the connection, close our connection to provider.
	//
	relay_outcome.await;
	warn!( "relay finished, closing connection" );

	provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to provider" );
}





// Test relaying messages
//
#[test]
//
fn relay_once()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );

	let exec = AsyncStd::default();
	let ex1  = exec.clone();
	let ex2  = exec.clone();
	let ex3  = exec.clone();


	let provider = async move
	{
		// Create mailbox for our handler
		//
		debug!( "start mailbox for handler" );
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );


		// get a framed connection
		//
		debug!( "start mailbox for provider" );
		let (peer_addr, _peer_evts) = peer_listen( ab, sm, &ex1 );

		drop( peer_addr );
		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		debug!( "start mailbox for consumer_to_relay" );

		let (mut to_relay, _)  = peer_connect( cb, &ex2, "consumer_to_relay" ).await;

		// Call the service and receive the response
		//
		let mut add  = remotes::Services::recipient::<Add >( to_relay.clone() );
		let mut show = remotes::Services::recipient::<Show>( to_relay.clone() );

		let resp = add.call( Add(5) ).await.expect( "Call failed" );
		assert_eq!( (), resp );

		add.send( Add(5) ).await.expect( "Send failed" );

		let resp = show.call( Show ).await.expect( "Call failed" );
		assert_eq!( 10, resp );

		warn!( "consumer end, telling relay to close connection" );

		to_relay.call( CloseConnection{ remote: false } ).await.expect( "close connection to relay" );

		warn!( "consumer end, relay processed CloseConnection" );
	};

	let relays = async move
	{
		relay( ba, bc, Box::pin( consumer ), true, &ex3 ).await;

		warn!( "relays end" );
	};

	block_on( join( provider, relays ) );
}



// Test relaying several relays deep
//
#[test]
//
fn relay_multi()
{
	// flexi_logger::Logger::with_str( "relay=trace, thespis_impl=info, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );
	let (cd, dc) = Endpoint::pair( 64, 64 );
	let (de, ed) = Endpoint::pair( 64, 64 );
	let (ef, fe) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();
	let ex3  = exec.clone();


	let provider = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );


		// get a framed connection
		//
		let _ = peer_listen( ab, sm, &ex1 );

		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		let (mut relay, _)  = peer_connect( fe, &ex2, "consumer_to_relay" ).await;

		// Call the service and receive the response
		//
		let mut add  = remotes::Services::recipient::<Add >( relay.clone() );
		let mut show = remotes::Services::recipient::<Show>( relay.clone() );

		let resp = add.call( Add(5) ).await.expect( "Call failed" );
		assert_eq!( (), resp );

		add.send( Add(5) ).await.expect( "Send failed" );

		let resp = show.call( Show ).await.expect( "Call failed" );
		assert_eq!( 10, resp );

		relay.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodeb" );
	};

	let relays = async move
	{
		let  relay4 = relay( ed, ef, Box::pin( consumer ), true, ex3.clone() )       ;
		let  relay3 = relay( dc, de, Box::pin( relay4   ), true, ex3.clone() )       ;
		let  relay2 = relay( cb, cd, Box::pin( relay3   ), true, ex3.clone() )       ;
		let _relay1 = relay( ba, bc, Box::pin( relay2   ), true, ex3         ).await ;
	};

	block_on( join( provider, relays ) );
}



// Test unknown service error in a relay
//
#[test]
//
fn relay_unknown_service()
{
	// flexi_logger::Logger::with_str( "relay=trace, thespis_impl=info, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();
	let ex3  = exec.clone();

	let provider = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );


		// get a framed connection
		//
		let _ = peer_listen( ab, sm, &ex1 );


		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		let (mut relay, _relay_evts) = peer_connect( cb, &ex2, "consumer_to_relay" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid        = Bytes::from( vec![ 5;16 ]);
		let cid: Bytes = ConnID::null().into();
		let msg: Bytes = serde_cbor::to_vec( &Add(5) ).unwrap().into();

		// This is the corrupt one that should trigger a deserialization error and close the connection
		//
		let mut buf = BytesMut::new();

		buf.extend( sid );
		buf.extend( cid );
		buf.extend( msg );

		let corrupt = MultiServiceImpl::try_from( Bytes::from( buf ) ).expect( "serialize Add(5)" );


		let rx = relay.call( Call::new( corrupt ) ).await

			.expect( "call peer" )
			.expect( "send out ms" )
		;

		assert_eq!
		(
			ConnectionError::UnknownService( vec![5;16] ),
			rx.await.expect( "return error, don't drop connection" ).unwrap_err()
		);


		relay.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodeb" );
	};


	let relay = relay( ba, bc, Box::pin( consumer ), true, ex3 );

	let provi_handle = exec.spawn_handle( provider ).expect( "Spawn provider"  );
	let relay_handle = exec.spawn_handle( relay    ).expect( "Spawn relays"  );

	block_on( join( provi_handle, relay_handle ) );
}




// Test behavior when a relay disconnects
//
#[test]
//
fn relay_disappeared()
{
	// flexi_logger::Logger::with_str( "pharos=trace, relay=trace, thespis_impl=info, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();
	let ex3  = exec.clone();


	let provider = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );

		// get a framed connection
		//
		let _ = peer_listen( ab, sm, &ex1 );


		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		let (mut relay, mut relay_evts) = peer_connect( cb, &ex2, "consumer_to_relay" ).await;

		let sid              = <Add as Service<remotes::Services>>::sid().clone();
		let bytes_sid: Bytes = sid.clone().into();
		let cid              = ConnID::default();
		let msg              = Bytes::from( vec![ 5;5 ] );


		let corrupt = MultiServiceImpl::create( sid, cid, msg );

		let rx = relay.call( Call::new( corrupt.clone() ) ).await

			.expect( "call peer" )
			.expect( "send out ms" )
		;

		assert_eq!
		(
			ConnectionError::Deserialize,
			rx.await.expect( "return error, don't drop connection" ).unwrap_err()
		);

		// TODO: These sometimes arrive in opposite order. Can we make the order deterministic? Should we?
		//
		// assert_eq!( Some( PeerEvent::RemoteError(ConnectionError::Deserialize) )                    , relay_evts.next().await );
		// assert_eq!( Some( PeerEvent::RemoteError(ConnectionError::ServiceGone(bytes_sid.to_vec())) ), relay_evts.next().await );

		for _ in 0..2usize
		{
			match relay_evts.next().await
			{
				Some(PeerEvent::RemoteError( ConnectionError::Deserialize      )) => {},
				Some(PeerEvent::RemoteError( ConnectionError::ServiceGone( s ) )) => assert_eq!( bytes_sid.to_vec(), s ) ,

				_ => unreachable!(),
			}
		}




		// The relay should have closed.
		// Send again
		//
		let rx = relay.call( Call::new( corrupt.clone() ) ).await

			.expect( "call peer" )
			.expect( "send out ms" )
		;

		// The service is no longer available
		//
		assert_eq!
		(
			ConnectionError::UnknownService( bytes_sid.to_vec() ),
			rx.await.expect( "return error, don't drop connection" ).unwrap_err()
		);


		// End the program
		//
		relay.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodeb" );
	};


	let relay = relay( ba, bc, Box::pin( consumer ), false, ex3 );

	let provi_handle = exec.spawn_handle( provider ).expect( "Spawn provider" );
	let relay_handle = exec.spawn_handle( relay    ).expect( "Spawn relay"    );

	block_on( join( provi_handle, relay_handle ) );
}




// Test behavior when a relay disconnects
//
#[test]
//
fn relay_disappeared_multi()
{
	// flexi_logger::Logger::with_str( "pharos=trace, relay=trace, thespis_impl=info, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );
	let (cd, dc) = Endpoint::pair( 64, 64 );
	let (de, ed) = Endpoint::pair( 64, 64 );
	let (ef, fe) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();
	let ex3  = exec.clone();

	let provider = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );

		// get a framed connection
		//
		let _ = peer_listen( ab, sm, &ex1 );


		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async move
	{
		let (mut relay, mut relay_evts) = peer_connect( fe, &ex2, "consumer_to_relay" ).await;

		let sid              = <Add as Service<remotes::Services>>::sid().clone();
		let bytes_sid: Bytes = sid.clone().into();
		let cid              = ConnID::default();
		let msg              = Bytes::from( vec![ 5;5 ] );

		let corrupt = MultiServiceImpl::create( sid, cid, msg );

		let rx = relay.call( Call::new( corrupt.clone() ) ).await

			.expect( "call peer" )
			.expect( "send out ms" )
		;

		assert_eq!
		(
			ConnectionError::Deserialize,
			rx.await.expect( "return error, don't drop connection" ).unwrap_err()
		);

		// TODO: These sometimes arrive in opposite order. Can we make the order deterministic? Should we?
		//
		// assert_eq!( Some( PeerEvent::RemoteError(ConnectionError::Deserialize) )                    , relay_evts.next().await );
		// assert_eq!( Some( PeerEvent::RemoteError(ConnectionError::ServiceGone(bytes_sid.to_vec())) ), relay_evts.next().await );

		for _ in 0..2usize
		{
			match relay_evts.next().await
			{
				Some(PeerEvent::RemoteError( ConnectionError::Deserialize    )) => {}                                  ,
				Some(PeerEvent::RemoteError( ConnectionError::ServiceGone(s) )) => assert_eq!( bytes_sid.to_vec(), s ) ,
				_                                                               => unreachable!()                      ,
			}
		}



		// The relay should have closed.
		// Send again
		//
		let rx = relay.call( Call::new( corrupt.clone() ) ).await

			.expect( "call peer" )
			.expect( "send out ms" )
		;

		// The service is no longer available
		//
		assert_eq!
		(
			ConnectionError::UnknownService( bytes_sid.to_vec() ),
			rx.await.expect( "return error, don't drop connection" ).unwrap_err()
		);

		// End the program
		//
		relay.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodeb" );
	};


	let relays = async move
	{
		let  relay4 = relay( ed, ef, Box::pin( consumer ), false, ex3.clone() )       ;
		let  relay3 = relay( dc, de, Box::pin( relay4   ), false, ex3.clone() )       ;
		let  relay2 = relay( cb, cd, Box::pin( relay3   ), false, ex3.clone() )       ;
		let _relay1 = relay( ba, bc, Box::pin( relay2   ), false, ex3         ).await ;
	};

	block_on( join( provider, relays ) );
}
