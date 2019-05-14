#![ feature( async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias, box_syntax, box_patterns, todo_macro, try_trait, optin_builtin_traits ) ]


// This currently combines testing error handling and peer events, because often the events are the way
// to be notified of errors.
//
// Tests:
//
// - ✔ Connection Closed events (closed by remote tested in basic_use)
// - ✔ Connection Closed events (send+call)
// - ✔ Header Deserialization (Remote)Error (Codec)
// - ✔ Invalid Codec
// - ✔ Header Unknown Service (Remote)Error
// - ✔ Service map Deserialization (Remote)Error
// - ✔ handling remote errors on call (let the caller know there were connection errors) -> tested in relay.rs
//
// - TODO: fuzz, SEND A WHOLE BUNCH OF BINARY DATA OVER THE NETWORK AND VERIFY THE CORRECT ERROR FOR EACH TYPE OF INPUT.


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// Test calling a remote service after the connection has closed.
//
#[test]
//
fn close_connection()
{
	let nodea = async
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
		let _ = listen_tcp( "127.0.0.1:20003", sm ).await;
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20003" ).await;

		// Close the connection and check the event
		//
		peera.send( peer::CloseConnection{ remote: false } ).await.expect( "Send CloseConnection" );

		assert_eq!( PeerEvent::Closed,  peera_evts.next().await.unwrap() );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}



// Test calling a remote service after the connection has closed.
//
#[test]
//
fn close_connection_call()
{
	let nodea = async
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
		let _ = listen_tcp( "127.0.0.1:20006", sm ).await;
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20006" ).await;

		// Close the connection and check the event
		//
		peera.call( peer::CloseConnection{ remote: false } ).await.expect( "Send CloseConnection" );

		assert_eq!( PeerEvent::Closed, peera_evts.next().await.unwrap() );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}



// Test Header Unknown Service (Remote)Error.
//
#[test]
//
fn header_unknown_service_error()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=trace, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let nodea = async
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
		let (_, mut evts) = listen_tcp( "127.0.0.1:20004", sm ).await;

		assert_eq!( PeerEvent::Error(ConnectionError::UnknownService( vec![3;16] )),  evts.next().await.unwrap() );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20004" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid = <Add as Service<remotes::Services>>::UniqueID::try_from( Bytes::from( vec![3;16] ) )

			.expect( "generate random sid" )
		;

		let ms  = MultiServiceImpl::create( sid, ConnID::null(), Codecs::CBOR, serde_cbor::to_vec( &Add(5) )

			.expect( "serialize Add(5)" ).into() )
		;

		peera.send( ms ).await.expect( "send ms to peera" );

		assert_eq!( PeerEvent::RemoteError(ConnectionError::UnknownService( vec![3;16] )),  peera_evts.next().await.unwrap() );

		peera.send( CloseConnection{ remote: false } ).await.expect( "close connection" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}




// Test Header Unknown Service (Remote)Error.
//
#[test]
//
fn header_deserialize()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=debug, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let nodea = async
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
		let (_, mut evts) = listen_tcp( "127.0.0.1:20007", sm ).await;

		assert_eq!( PeerEvent::Error(ConnectionError::Deserialize),  evts.next().await.unwrap() );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20007" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid: Bytes = <Add as Service<remotes::Services>>::sid().clone().into();
		let cid: Bytes = ConnID::null().into();
		let msg: Bytes = serde_cbor::to_vec( &Add(5) ).unwrap().into();

		// This is the corrupt one that should trigger a deserialization error and close the connection
		//
		let cod = Bytes::from( vec![ 1,2,3,4 ] );

		let mut buf = BytesMut::new();

		buf.extend( sid );
		buf.extend( cid );
		buf.extend( cod );
		buf.extend( msg );

		let ms  = MultiServiceImpl::try_from( Bytes::from( buf ) ).expect( "serialize Add(5)" );

		peera.call( ms ).await.expect( "send ms to peera" );
		assert_eq!( PeerEvent::RemoteError(ConnectionError::Deserialize), peera_evts.next().await.unwrap() );

		peera.send( CloseConnection{ remote: false } ).await.expect( "close connection" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}




// Test Invalid codec error.
//
#[test]
//
fn invalid_codec()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=debug, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let nodea = async
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
		let (_, mut evts) = listen_tcp( "127.0.0.1:20008", sm ).await;

		let cod: Bytes = Codecs::UTF8.into();
		assert_eq!( PeerEvent::Error(ConnectionError::UnsupportedCodec(cod.to_vec())),  evts.next().await.unwrap() );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20008" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid: Bytes = <Add as Service<remotes::Services>>::sid().clone().into();
		let cid: Bytes = ConnID::null().into();
		let msg: Bytes = serde_cbor::to_vec( &Add(5) ).unwrap().into();

		// Only CBOR supported
		//
		let cod: Bytes = Codecs::UTF8.into();
		let co2        = cod.clone();

		let mut buf = BytesMut::new();

		buf.extend( sid );
		buf.extend( cid );
		buf.extend( cod );
		buf.extend( msg );

		let ms = MultiServiceImpl::try_from( Bytes::from( buf ) ).expect( "serialize Add(5)" );

		peera.call( ms ).await.expect( "send ms to peera" );
		assert_eq!( PeerEvent::RemoteError(ConnectionError::UnsupportedCodec(co2.to_vec())), peera_evts.next().await.unwrap() );

		peera.send( CloseConnection{ remote: false } ).await.expect( "close connection" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}



// Test Service map Deserialization (Remote)Error.
//
#[test]
//
fn sm_deserialize_error()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=trace, thespis_impl_remote=trace, tokio=warn" ).start().unwrap();

	let nodea = async
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
		let (_, mut evts) = listen_tcp( "127.0.0.1:20005", sm ).await;

		assert_eq!( PeerEvent::Error(ConnectionError::Deserialize),  evts.next().await.unwrap() );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20005" ).await;

		// Create some random data that shouldn't deserialize
		//
		let sid = <Add as Service<remotes::Services>>::sid().clone();
		let ms  = MultiServiceImpl::create( sid, ConnID::null(), Codecs::CBOR, Bytes::from( vec![3,3]));

		peera.send( ms ).await.expect( "send ms to peera" );

		assert_eq!( PeerEvent::RemoteError(ConnectionError::Deserialize),  peera_evts.next().await.unwrap() );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}
