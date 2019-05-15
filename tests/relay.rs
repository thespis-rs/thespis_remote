#![ feature( async_await, box_syntax ) ]



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
async fn relay( connect: &'static str, listen: &'static str, next: Pin<Box< dyn Future< Output=() >>>, relay_show: bool )
{
	let (mut peera_addr, peera_evts) = connect_to_tcp( connect ).await;
	let peera_addr2                  = peera_addr.clone();

	// Relay part ---------------------

	let relay = async move
	{
		let (srv_sink, srv_stream) = listen_tcp_stream( listen ).await;

		// Create mailbox for peer
		//
		let mb_peer  : Inbox<MyPeer> = Inbox::new()                  ;
		let peer_addr                = Addr ::new( mb_peer.sender() );

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::new( peer_addr, srv_stream.compat(), srv_sink.sink_compat() ).expect( "spawn peer" );

		let add  = <Add   as Service<remotes::Services>>::sid();
		let show = <Show  as Service<remotes::Services>>::sid();

		let relayed = if relay_show
		{
			vec![ add, show ]
		}

		else { vec![ add ] };

		peer.register_relayed_services( relayed, peera_addr2, peera_evts ).expect( "register relayed" );

		mb_peer.start_fut( peer ).await;
	};


	let (relay_fut, relay_outcome) = relay.remote_handle();
	rt::spawn( relay_fut ).expect( "failed to spawn server" );

	// we need to spawn this after peerb, otherwise peerb is not listening yet when we try to connect.
	//
	rt::spawn( next ).expect( "Spawn next"  );


	// If the nodec closes the connection, close our connection to peera.
	//
	relay_outcome.await;
	peera_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to provider" );
}





// Test relaying messages
//
#[test]
//
fn relay_once()
{
	// flexi_logger::Logger::with_str( "remotes=trace, thespis_impl=trace, tokio=warn" ).start().unwrap();

	let provider = async
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );


		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:20000", sm ).await;

		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async
	{
		let (mut relay, _)  = connect_to_tcp( "127.0.0.1:30000" ).await;

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

	let relays = async
	{
		relay( "127.0.0.1:20000", "127.0.0.1:30000", Box::pin( consumer ), true ).await;
	};

	rt::spawn( provider  ).expect( "Spawn provider" );
	rt::spawn( relays    ).expect( "Spawn relays"   );

	rt::run();
}



// Test relaying several relays deep
//
#[test]
//
fn relay_multi()
{
	// flexi_logger::Logger::with_str( "relay=trace, thespis_impl=info, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let provider = async
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );


		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:30010", sm ).await;

		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async
	{
		let (mut relay, _)  = connect_to_tcp( "127.0.0.1:30014" ).await;

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

	let relays = async
	{
		let  relay4 = relay( "127.0.0.1:30013", "127.0.0.1:30014", Box::pin( consumer ), true )      ;
		let  relay3 = relay( "127.0.0.1:30012", "127.0.0.1:30013", Box::pin( relay4   ), true )      ;
		let  relay2 = relay( "127.0.0.1:30011", "127.0.0.1:30012", Box::pin( relay3   ), true )      ;
		let _relay1 = relay( "127.0.0.1:30010", "127.0.0.1:30011", Box::pin( relay2   ), true ).await;
	};

	rt::spawn( provider  ).expect( "Spawn provider" );
	rt::spawn( relays    ).expect( "Spawn relays"   );

	rt::run();
}



// Test unknown service error in a relay
//
#[test]
//
fn relay_unknown_service()
{
	// flexi_logger::Logger::with_str( "relay=trace, thespis_impl=info, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let provider = async
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );


		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:30015", sm ).await;


		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async
	{
		let (mut relay, _relay_evts) = connect_to_tcp( "127.0.0.1:30016" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid        = Bytes::from( vec![ 5;16 ]);
		let cid: Bytes = ConnID::null().into();
		let msg: Bytes = serde_cbor::to_vec( &Add(5) ).unwrap().into();

		// This is the corrupt one that should trigger a deserialization error and close the connection
		//
		let cod: Bytes = Codecs::CBOR.into();

		let mut buf = BytesMut::new();

		buf.extend( sid );
		buf.extend( cid );
		buf.extend( cod );
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


	let relay = relay( "127.0.0.1:30015", "127.0.0.1:30016", Box::pin( consumer ), true );

	rt::spawn( provider ).expect( "Spawn provider" );
	rt::spawn( relay    ).expect( "Spawn relays"   );

	rt::run();
}




// Test behavior when a relay disconnects
//
#[test]
//
fn relay_disappeared()
{
	// flexi_logger::Logger::with_str( "pharos=trace, relay=trace, thespis_impl=info, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let provider = async
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );

		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:30017", sm ).await;


		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async
	{
		let (mut relay, mut relay_evts) = connect_to_tcp( "127.0.0.1:30018" ).await;

		let sid              = <Add as Service<remotes::Services>>::sid().clone();
		let bytes_sid: Bytes = sid.clone().into();
		let cid              = ConnID::default();
		let cod              = Codecs::CBOR;
		let msg              = Bytes::from( vec![ 5;5 ] );


		let corrupt = MultiServiceImpl::create( sid, cid, cod, msg );

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

		for _ in 0..2
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


	let relay = relay( "127.0.0.1:30017", "127.0.0.1:30018", Box::pin( consumer ), false );

	rt::spawn( provider ).expect( "Spawn provider" );
	rt::spawn( relay    ).expect( "Spawn relays"   );

	rt::run();
}




// Test behavior when a relay disconnects
//
#[test]
//
fn relay_disappeared_multi()
{
	// flexi_logger::Logger::with_str( "pharos=trace, relay=trace, thespis_impl=info, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let provider = async
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );

		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:30019", sm ).await;


		trace!( "End of provider" );
	};


	// --------------------------------------

	let consumer = async
	{
		let (mut relay, mut relay_evts) = connect_to_tcp( "127.0.0.1:30023" ).await;

		let sid              = <Add as Service<remotes::Services>>::sid().clone();
		let bytes_sid: Bytes = sid.clone().into();
		let cid              = ConnID::default();
		let cod              = Codecs::CBOR;
		let msg              = Bytes::from( vec![ 5;5 ] );


		let corrupt = MultiServiceImpl::create( sid, cid, cod, msg );

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

		for _ in 0..2
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


	let relays = async
	{
		let  relay4 = relay( "127.0.0.1:30022", "127.0.0.1:30023", Box::pin( consumer ), false )      ;
		let  relay3 = relay( "127.0.0.1:30021", "127.0.0.1:30022", Box::pin( relay4   ), false )      ;
		let  relay2 = relay( "127.0.0.1:30020", "127.0.0.1:30021", Box::pin( relay3   ), false )      ;
		let _relay1 = relay( "127.0.0.1:30019", "127.0.0.1:30020", Box::pin( relay2   ), false ).await;
	};


	rt::spawn( provider ).expect( "Spawn provider" );
	rt::spawn( relays   ).expect( "Spawn relays"   );

	rt::run();
}
