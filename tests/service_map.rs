#![ feature( async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias, box_syntax, box_patterns, todo_macro, try_trait, optin_builtin_traits ) ]


mod common;

use common::*                                  ;
use common::import::{ *, assert_eq, assert_ne };

mod a
{
	use crate::*;

	service_map!
	(
		namespace:     remotes   ;
		peer_type:     MyPeer    ;
		multi_service: MS        ;
		services     : Add, Show ;
	);


	service_map!
	(
		namespace:     others  ;
		peer_type:     MyPeer  ;
		multi_service: MS      ;
		services     : Add     ;
	);
}

mod b
{
	use crate::*;

	service_map!
	(
		namespace:     remotes ;
		peer_type:     MyPeer  ;
		multi_service: MS      ;
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


// Verify that the same service, in a different namespace has different service id.
//
#[ test ]
//
fn sid_diff_for_diff_service()
{
	assert_ne!( <Add as Service<a::remotes::Services>>::sid(), <Show as Service<a::remotes::Services>>::sid() );
}


// Verify that the same service in different service maps with the same namespace has identical sid
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
	// Create mailbox for our handler
	//
	let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );

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

