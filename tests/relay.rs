#![ feature( async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias, box_syntax, box_patterns, todo_macro, try_trait, optin_builtin_traits ) ]


// TODO:
//
// - ✔ Basic relaying
// - Relay dissapeared event
// - try to send after relay has dissapeared (should give mailboxclosed)


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// Test relaying messages
//
#[test]
//
fn relay()
{
	// flexi_logger::Logger::with_str( "remotes=trace, thespis_impl=trace, tokio=warn" ).start().unwrap();

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
		let _ = listen_tcp( "127.0.0.1:20000", sm ).await;

		trace!( "End of nodea" );
	};



	let nodeb = async
	{
		let (sink_b, stream_b) = connect_return_stream( "127.0.0.1:20000" ).await;

		// Create mailbox for peer
		//
		let     mb_peera   : Inbox<MyPeer> = Inbox::new()                  ;
		let mut peera_addr                 = Addr ::new( mb_peera.sender() );

		// create peer with stream/sink
		//
		let mut peera = Peer::new( peera_addr.clone(), stream_b.compat(), sink_b.sink_compat() )

			.expect( "spawn peera" )
		;


		let     peera2  = peera_addr.clone();
		let peera_evts  = peera.observe( 10 );


		mb_peera.start( peera ).expect( "spawn peera" );


		// Relay part ---------------------

		let relay = async move
		{
			let (srv_sink, srv_stream) = listen_tcp_stream( "127.0.0.1:30000" ).await;


			// Create mailbox for peer
			//
			let mb_peer  : Inbox<MyPeer> = Inbox::new()                  ;
			let peer_addr                = Addr ::new( mb_peer.sender() );

			// create peer with stream/sink + service map
			//
			let mut peer = Peer::new( peer_addr, srv_stream.compat(), srv_sink.sink_compat() ).expect( "spawn peer" );

			let add  = <Add   as Service<remotes::Services>>::sid();
			let show = <Show  as Service<remotes::Services>>::sid();

			peer.register_relayed_services( vec![ add, show ], peera2, peera_evts ).expect( "register relayed" );

			mb_peer.start_fut( peer ).await;
		};


		let (relay_fut, relay_outcome) = relay.remote_handle();
		rt::spawn( relay_fut ).expect( "failed to spawn server" );

		// --------------------------------------

		let nodec = async
		{
			let (mut peerb, _)  = connect_to_tcp( "127.0.0.1:30000" ).await;

			// Call the service and receive the response
			//
			let mut add  = remotes::Services::recipient::<Add >( peerb.clone() );
			let mut show = remotes::Services::recipient::<Show>( peerb.clone() );

			let resp = add.call( Add(5) ).await.expect( "Call failed" );
			assert_eq!( (), resp );

			add.send( Add(5) ).await.expect( "Send failed" );

			let resp = show.call( Show ).await.expect( "Call failed" );
			assert_eq!( 10, resp );

			peerb.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodeb" );
		};

		// we need to spawn this after peerb, otherwise peerb is not listening yet when we try to connect.
		//
		rt::spawn( nodec ).expect( "Spawn nodec"  );


		// If the nodec closes the connection, close our connection to peera.
		//
		relay_outcome.await;
		peera_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodea" );
	};

	rt::spawn( nodea  ).expect( "Spawn nodea"  );
	rt::spawn( nodeb  ).expect( "Spawn nodeb"  );

	rt::run();
}
