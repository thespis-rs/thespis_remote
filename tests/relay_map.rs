// Tests TODO:
//
// - ... a bunch of stuff to be written ...
// ✔ use with addr -> already tested in relay.rs
// ✔ test a load balancing scenario


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };


// Test relaying messages
//
#[async_std::test]
//
async fn relay_once_load_balance()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (bc, cb) = Endpoint::pair( 64, 64 );

	let (provider_cx , provider_handle ) = provider( Some( "provider1".into() ), AsyncStd ).await;
	let (provider_cx2, provider_handle2) = provider( Some( "provider1".into() ), AsyncStd ).await;


	// --------------------------------------

	let consumer = async move
	{
		debug!( "start mailbox for consumer_to_relay" );

		let (mut to_relay, _)  = peer_connect( cb, AsyncStd, "consumer_to_relay" ).await;

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_relay.clone() );

		// This is sending in round robin, but the service saves state in between Add and Show, so for
		// 2 providers, do this exactly twice so each get's the same requests...
		//
		// This tells us that we don't send to the same one all the time.
		//
		for _ in 0..2
		{
			assert_eq!( addr.call( Add(5) ).await, Ok(()) );

			addr.send( Add(5) ).await.expect( "Send failed" );
		}

		for _ in 0..2
		{
			let resp = addr.call( Show ).await.expect( "Call failed" );

			// TODO: This assert sometimes fails with 5, so an addition above didn't happen but didn't return an error.
			//       To detect we should run in a loop with trace and debug.
			//
			assert_eq!( 10, resp );
		}


		warn!( "consumer end, telling relay to close connection" );

		to_relay.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );

		warn!( "consumer end, relay processed CloseConnection" );
	};

	let relays = async move
	{
		relay_closure( vec![ provider_cx, provider_cx2 ], bc, Box::pin( consumer ), true, AsyncStd ).await;

		warn!( "relays end" );
	};

	join3( provider_handle, provider_handle2, relays ).await;
}



// Test debug implementation.
// Fixes the output of the debug implementation. Mainly, this fixes the sid impl. If sid's change,
// that would be a breaking change, because people might be counting on them, especially if there
// would be programs written in other languages, because those would manually implement the wire protocol.
// Thus if this changes, you should bump a breaking change version.
//
#[async_std::test]
//
async fn debug()
{
	let (_, cx) = Endpoint::pair( 64, 64 );

	// Create mailbox for peer
	//
	let (peer, mb, peer_addr) = CborWF::create_peer( "relay_to_consumer", cx, 1024, 1024, AsyncStd, None, None ).expect( "spawn peer" );
	let id = peer_addr.id()                                            ;

	AsyncStd.spawn( mb.start(peer).map(|_|()) ).expect( "Start mailbox of Peer" );


	let add  = <Add  as remotes::Service>::sid();
	let show = <Show as remotes::Service>::sid();

	let rm = RelayMap::new( ServiceHandler::Address( Box::new(peer_addr) ), vec![ add, show ] );

	// All tests from the same file seem to run in the same process, so sometimes
	// if the test for clone has run first, the ID will be 1.
	//
	// TODO: output service names.
	//
	let txt = format!
("RelayMap, handler: ServiceHandler: Address: id: {}, name: \"relay_to_consumer\", services:
{{
	sid: 0x6440cfd17c374646
	sid: 0xcdd3781867767588
}}",
&id,
);

	assert_eq!( txt, format!( "{:?}", rm ) );
}
