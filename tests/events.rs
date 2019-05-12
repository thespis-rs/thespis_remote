#![ feature( await_macro, async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias, box_syntax, box_patterns, todo_macro, try_trait, optin_builtin_traits ) ]


// This currently combines testing error handling and peer events, because often the events are the way
// to be notified of errors.
//
// TODO:
//
// - ✔ Connection Closed events -> closed by remote tested in basic_use
// - Header Deserialization (Remote)Error
// - ✔ Header Unknown Service (Remote)Error
// - ✔ Service map Deserialization (Remote)Error
// - all errors in send_service and call_service in the macro. A lot of things can go wrong.
// - handling remote errors on call (let the caller know there wer connection errors)
// - Same errors on call instead of Send
//
// - SEND A WHOLE BUNCH OF BINARY DATA OVER THE NETWORK AND VERIFY THE CORRECT ERROR FOR EACH TYPE OF INPUT.


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
		let _ = await!( listen_tcp( "127.0.0.1:20003", sm ) );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = await!( connect_to_tcp( "127.0.0.1:20003" ) );

		// Close the connection and check the event
		//
		await!( peera.send( peer::CloseConnection{ remote: false } ) ).expect( "Send CloseConnection" );

		assert_eq!( PeerEvent::Closed, await!( peera_evts.next() ).unwrap() );
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
		let (_, mut evts) = await!( listen_tcp( "127.0.0.1:20004", sm ) );

		assert_eq!( PeerEvent::Error(ConnectionError::UnknownService( vec![3;16] )), await!( evts.next() ).unwrap() );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = await!( connect_to_tcp( "127.0.0.1:20004" ) );

		// Create some random data that shouldn't deserialize
		//
		let sid = <Add as Service<remotes::Services>>::UniqueID::try_from( Bytes::from( vec![3;16] ) )

			.expect( "generate random sid" )
		;

		let ms  = MultiServiceImpl::create( sid, ConnID::null(), Codecs::CBOR, serde_cbor::to_vec( &Add(5) )

			.expect( "serialize Add(5)" ).into() )
		;

		await!( peera.send( ms ) ).expect( "send ms to peera" );

		assert_eq!( PeerEvent::RemoteError(ConnectionError::UnknownService( vec![3;16] )), await!( peera_evts.next() ).unwrap() );

		await!( peera.send( CloseConnection{ remote: false } ) ).expect( "close connection" );
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
		let (_, mut evts) = await!( listen_tcp( "127.0.0.1:20005", sm ) );

		assert_eq!( PeerEvent::Error(ConnectionError::Deserialize), await!( evts.next() ).unwrap() );
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = await!( connect_to_tcp( "127.0.0.1:20005" ) );

		// Create some random data that shouldn't deserialize
		//
		let sid = <Add as Service<remotes::Services>>::sid().clone();
		let ms  = MultiServiceImpl::create( sid, ConnID::null(), Codecs::CBOR, Bytes::from( vec![3,3]));

		await!( peera.send( ms ) ).expect( "send ms to peera" );

		assert_eq!( PeerEvent::RemoteError(ConnectionError::Deserialize), await!( peera_evts.next() ).unwrap() );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}
