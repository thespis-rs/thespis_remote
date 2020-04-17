#![ cfg( feature = "futures_codec" ) ]

// Tested:
//
// - ✔ Verify that the same      service, in a   different namespace has different service id.
// - ✔ Verify that a   different service, in the same      namespace has different service id.
// - ✔ Verify that the same      service, in the same      namespace but in different servicemap has identical sid
// - ✔ Test clone.
// - ✔ Test Debug.
// - Test ServiceID::Debug
// - Test adding services at runtime.
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
	assert_ne!( <Add as a::remotes::Service>::sid(), <Add as a::others::Service>::sid() );
}


// Verify that a different service, in the same namespace has different service id.
//
#[ test ]
//
fn sid_diff_for_diff_service()
{
	assert_ne!( <Add as a::remotes::Service>::sid(), <Show as a::remotes::Service>::sid() );
}


// Verify that the same service in different servicemaps with the same namespace has identical sid
//
#[ test ]
//
fn sid_same_for_same_ns()
{
	assert_eq!( <Add as a::remotes::Service>::sid(), <Add as b::remotes::Service>::sid() );
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
	let addr_handler = Addr::builder().start( Sum(0), &exec ).expect( "spawn actor mailbox" );

	// Create a service map
	//
	let mut sm = remotes::Services::new( exec );

	// Register our handlers
	//
	sm.register_handler::<Add >( addr_handler.clone_box() );
	sm.register_handler::<Show>( addr_handler.clone_box() );

	let serial = format!( "{:?}", sm );

	assert_eq!( serial, format!( "{:?}", sm ) );
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
	// Create mailbox for peer
	//
	let (sum_addr, _) = Addr::<Sum>::builder().name( "for_debug".into() ).build();

	let mut sm = remotes::Services::new( AsyncStd );

	sm.register_handler::<Add >( sum_addr.clone_box() );
	sm.register_handler::<Show>( sum_addr.clone_box() );

	// All tests from the same file seem to run in the same process, so sometimes
	// if the test for clone has run first, the ID will be 1.
	//
	let id = sum_addr.id();

	let txt = format!
("remotes::Services
{{
	Add  - sid: 0xbcc09d3812378e171ad366d75f687757 - handler: id({}), name(for_debug)
	Sub  - sid: 0xbcc09d3812378e179c6a3582c2729a7c - handler: none
	Show - sid: 0xbcc09d3812378e17e1a1e89b512c025a - handler: id({}), name(for_debug)
}}",
&id,
&id,
);

	assert_eq!( txt, format!( "{:?}", sm ) );
}



// Test debug implementation of ServiceID
//
#[test]
//
fn debug_service_id()
{
	// Unfortunately we can only register the service in Services::new. TODO: document this.
	//
	let _sm = remotes::Services::new( AsyncStd );

	use remotes::Service;

	assert_eq!( "ServiceID: remotes::Add (0xbcc09d3812378e171ad366d75f687757)", format!( "{:?}", Add::sid() ) );
}


