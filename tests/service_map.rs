// Tested:
//
// - ✔ Verify that the same      service, in a   different namespace has different service id.
// - ✔ Verify that a   different service, in the same      namespace has different service id.
// - ✔ Verify that the same      service, in the same      namespace but in different servicemap has identical sid
// - ✔ Test clone.
// - ✔ Test Debug.
//


mod common;

use common::*                                  ;
use common::import::{ *, assert_eq, assert_ne };

mod a
{
	use crate::*;

	service_map!
	(
		namespace:     remotes   ;
		services     : Add, Show ;
	);


	service_map!
	(
		namespace:     others  ;
		services     : Add     ;
	);
}

mod b
{
	use crate::*;

	service_map!
	(
		namespace:     remotes ;
		services     : Add     ;
	);
}


// Verify that the same service, in a different namespace has different service id.
//
#[ test ]
//
fn sid_diff_for_diff_ns()
{
	assert_ne!( <Add as Service<a::remotes::Services>>::sid(), <Add as Service<a::others::Services>>::sid() );
}


// Verify that a different service, in the same namespace has different service id.
//
#[ test ]
//
fn sid_diff_for_diff_service()
{
	assert_ne!( <Add as Service<a::remotes::Services>>::sid(), <Show as Service<a::remotes::Services>>::sid() );
}


// Verify that the same service in different servicemaps with the same namespace has identical sid
//
#[ test ]
//
fn sid_same_for_same_ns()
{
	assert_eq!( <Add as Service<a::remotes::Services>>::sid(), <Add as Service<b::remotes::Services>>::sid() );
}


// Test clone.
//
#[test]
//
fn clone()
{
	let exec = ThreadPool::new().expect( "create threadpool" );

	// Create mailbox for our handler
	//
	let addr_handler = Addr::try_from( Sum(0), &exec ).expect( "spawn actor mailbox" );

	// Create a service map
	//
	let mut sm = remotes::Services::new();

	// Register our handlers
	//
	sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
	sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );

	let serial = format!( "{:?}", sm );

	assert_eq!( serial, format!( "{:?}", sm.clone() ) );
}



// Test debug implementation.
// Fixes the output of the debug implementation. Mainly, this fixes the sid impl. If sid's change,
// that would be a breaking change, because people might be counting on them, especially if there
// would be programs written in other languages, because those would manually implement the wire protocol.
// Thus if this changes, you should bump a breaking change version.
//
#[test]
//
fn debug()
{
	let exec = ThreadPool::new().expect( "create threadpool" );

	// Create mailbox for our handler
	//
	let addr_handler = Addr::try_from( Sum(0), &exec ).expect( "spawn actor mailbox" );

	// Create a service map
	//
	let mut sm = remotes::Services::new();
	// Register our handlers
	//
	sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
	sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );

	// All tests from the same file seem to run in the same process, so sometimes
	// if the test for clone has run first, the ID will be 1.
	//
	let id = < Addr<Sum> as Recipient<Add> >::actor_id( &addr_handler );

	let txt = format!
("remotes::Services
{{
	Add  - sid: c5c22cab4f2d334e0000000000000000 - handler (actor_id): {:?}
	Show - sid: 0617b1cfb55b99700000000000000000 - handler (actor_id): {:?}
}}",
&id,
&id,
);

	assert_eq!( txt, format!( "{:?}", sm ) );
}


