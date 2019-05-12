#![ feature( async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias, box_syntax, box_patterns, todo_macro, try_trait, slice_concat_ext ) ]

mod common;
use common::*;

fn main()
{
	let _handle = flexi_logger::Logger::with_str( "thespis_impl=debug, tokio=info" ).start().unwrap();

	let program = async move
	{
		trace!( "Starting peerC" );

		let (mut peerb, _peerb_evts) = connect_to_tcp( "127.0.0.1:8999" ).await;


		// Call the service and receive the response
		//
		let mut service_a = peer_a::Services::recipient::<ServiceA>( peerb.clone() );
		let mut service_b = peer_a::Services::recipient::<ServiceB>( peerb.clone() );



		let resp = service_a.call( ServiceA{ msg: "hi from peerC".to_string() } ).await.expect( "Call failed" );

		dbg!( resp );



		// Send
		//
		service_b.send( ServiceB{ msg: "SEND from peerC".to_string() } ).await.expect( "Send failed" );



		let resp = service_a.call( ServiceA{ msg: "hi from peerC -- again!!!".to_string() } ).await.expect( "Call failed" );

		dbg!( resp );


		// Send
		//
		service_b.send( ServiceB{ msg: "SEND AGAIN from peerC".to_string() } ).await.expect( "Send failed" );

		peerb.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );
	};

	rt::spawn( program ).expect( "Spawn program" );

	rt::run();
}
