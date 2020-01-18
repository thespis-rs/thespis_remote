#![ cfg( feature = "futures_codec" ) ]



// Tests TODO:
//
// ✔ start with empty and add later
// ✔ adding should start providing
// ✔ add empty list
// ✔ removing service should stop providing
// ✔ remove non existing
// ✔ updating should start delivering to new handler
//
mod common;

use common::*                       ;
use common::import::{ *, assert_eq };
use remotes::Service;



// start with empty and add later.
//
#[test]
//
fn start_empty()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();


	let test = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let sm = Arc::new( remotes::Services::new() );


		// get a framed connection
		//
		let (mut provider_addr, _, provider_handle) = peer_listen( server, sm.clone(), ex1.clone(), "peera" );


		let (to_provider, _)  = peer_connect( client, ex2, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_provider.clone() );

		// Register our handlers
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.clone_box() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.clone_box() ) );

		provider_addr.call( AddServices{ sm } ).await.expect( "Add service Add, Show" );

		let resp = addr.call( Add(5) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 5, resp );

		provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );

		drop( provider_addr );
		provider_handle.await;

		trace!( "end of test" );
	};


	block_on( test );
}


// Verify a service is not present, add it and use it.
//
#[test]
//
fn adding_starts_providing()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();


	let test = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let sm = Arc::new( remotes::Services::new() );
		// Register our handlers
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.clone_box() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.clone_box() ) );


		// get a framed connection
		//
		let (mut provider_addr, _, provider_handle) = peer_listen( server, sm.clone(), ex1.clone(), "peera" );


		let (to_provider, _)  = peer_connect( client, ex2, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_provider.clone() );

		let resp = addr.call( Add(5) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		let resp = addr.call( Sub(1) ).await;
		assert_matches!( resp.unwrap_err(), ThesRemoteErr::Remote{ err: ConnectionError::UnknownService{..}, .. } );


		let sm2 = Arc::new( remotes::Services::new() );
		sm2.register_handler::<Sub >( Receiver::new( addr_handler.clone_box() ) );

		provider_addr.call( AddServices{ sm: sm2 } ).await.expect( "Add service Sub" );

		let resp = addr.call( Sub(1) ).await.expect( "Call Sub(1)" );
		assert_eq!( (), resp );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 4, resp );

		provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );

		drop( provider_addr );
		provider_handle.await;

		trace!( "end of test" );
	};

	block_on( test );
}


// Adding an empty list should have no impact.
//
#[test]
//
fn add_empty()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();


	let test = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let sm = Arc::new( remotes::Services::new() );
		// Register our handlers
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.clone_box() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.clone_box() ) );


		// get a framed connection
		//
		let (mut provider_addr, _, provider_handle) = peer_listen( server, sm.clone(), ex1.clone(), "peera" );


		let (to_provider, _)  = peer_connect( client, ex2, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_provider.clone() );

		let sm2 = Arc::new( remotes::Services::new() );
		provider_addr.call( AddServices{ sm: sm2 } ).await.expect( "Add service Sub" );

		let resp = addr.call( Add(5) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 5, resp );

		provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );

		drop( provider_addr );
		provider_handle.await;

		trace!( "end of test" );
	};

	block_on( test );
}


// Remove should stop providing.
//
#[test]
//
fn remove_should_stop_providing()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();


	let test = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let sm = Arc::new( remotes::Services::new() );
		// Register our handlers
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.clone_box() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.clone_box() ) );


		// get a framed connection
		//
		let (mut provider_addr, _, provider_handle) = peer_listen( server, sm.clone(), ex1.clone(), "peera" );


		let (to_provider, _)  = peer_connect( client, ex2, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_provider.clone() );

		let resp = addr.call( Add(5) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		provider_addr.call( RemoveServices{ services: vec![ Add::sid().clone() ] } ).await.expect( "Add service Sub" );

		let resp = addr.call( Add(5) ).await;
		assert_matches!( resp.unwrap_err(), ThesRemoteErr::Remote{ err: ConnectionError::UnknownService{..}, .. } );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 5, resp );

		provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );

		drop( provider_addr );
		provider_handle.await;

		trace!( "end of test" );
	};


	block_on( test );
}



// Remove non-existing = noop.
//
#[test]
//
fn remove_non_existing()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();


	let test = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let sm = Arc::new( remotes::Services::new() );
		// Register our handlers
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.clone_box() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.clone_box() ) );


		// get a framed connection
		//
		let (mut provider_addr, _, provider_handle) = peer_listen( server, sm.clone(), ex1.clone(), "peera" );


		let (to_provider, _)  = peer_connect( client, ex2, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_provider.clone() );

		let resp = addr.call( Add(5) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		provider_addr.call( RemoveServices{ services: vec![ Sub::sid().clone() ] } ).await.expect( "Add service Sub" );

		let resp = addr.call( Add(1) ).await.expect( "Call Sub(1)" );
		assert_eq!( (), resp );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 6, resp );

		provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );

		drop( provider_addr );
		provider_handle.await;

		trace!( "end of test" );
	};

	block_on( test );
}



// Update to new handler.
//
#[test]
//
fn update_to_new_handler()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let exec = ThreadPool::new().expect( "create threadpool" );
	let ex1  = exec.clone();
	let ex2  = exec.clone();


	let test = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler  = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );
		let addr_handler2 = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let sm = Arc::new( remotes::Services::new() );
		// Register our handlers
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.clone_box() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.clone_box() ) );

		// Create a service map
		//
		let sm2 = Arc::new( remotes::Services::new() );
		// Register our handlers
		//
		sm2.register_handler::<Add >( Receiver::new( addr_handler2.clone_box() ) );
		sm2.register_handler::<Show>( Receiver::new( addr_handler2.clone_box() ) );

		// get a framed connection
		//
		let (mut provider_addr, _, provider_handle) = peer_listen( server, sm.clone(), ex1.clone(), "peera" );


		let (to_provider, _)  = peer_connect( client, ex2, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_provider.clone() );

		let resp = addr.call( Add(5) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 5, resp );


		provider_addr.call( AddServices{ sm: sm2 } ).await.expect( "Add new handler" );


		let resp = addr.call( Add(3) ).await.expect( "Call Add(5)" );
		assert_eq!( (), resp );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 3, resp );


		provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );

		drop( provider_addr );
		provider_handle.await;

		trace!( "end of test" );
	};

	block_on( test );
}
