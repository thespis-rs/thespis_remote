#![ feature( async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias, box_syntax, box_patterns, todo_macro, try_trait, optin_builtin_traits ) ]


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };


// Tests:
//
// - ✔ basic remote funcionality: intertwined sends and calls.
// - ✔ correct async behavior: verify that a peer can continue to send/receive while waiting for the response to a call.
// - ✔ call a remote service after the connection has closed: verify peer event and error kind.



// Test basic remote funcionality. Test intertwined sends and calls.
//
#[test]
//
fn remote()
{
	let peera = async
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

		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:8998", sm ).await;
	};


	let peerb = async
	{
		let (mut peera, _)  = connect_to_tcp( "127.0.0.1:8998" ).await;

		// Call the service and receive the response
		//
		let mut add  = remotes::Services::recipient::<Add >( peera.clone() );
		let mut show = remotes::Services::recipient::<Show>( peera.clone() );

		let resp = add.call( Add(5) ).await.expect( "Call failed" );
		assert_eq!( (), resp );

		add.send( Add(5) ).await.expect( "Send failed" );

		let resp = show.call( Show ).await.expect( "Call failed" );
		assert_eq!( 10, resp );

		peera.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( peera  ).expect( "Spawn peera"  );
	rt::spawn( peerb  ).expect( "Spawn peerb"  );

	rt::run();
}




#[ derive( Actor ) ]
//
pub struct Parallel
{
	pub sum: Box< Recipient<Show> >,
}


impl Handler< Show > for Parallel
{
	fn handle( &mut self, _: Show ) -> ReturnNoSend<u64> { Box::pin( async move
	{
		self.sum.call( Show ).await.expect( "call sum" )
	})}
}


service_map!
(
	namespace     : parallel ;
	peer_type     : MyPeer   ;
	multi_service : MS       ;
	services      : Show     ;
);




// Test correct async behavior. Verify that a peer can continue to
// send/receive while waiting for the response to a call.
//
#[test]
//
fn parallel()
{
	let peera = async
	{
		// get a framed connection
		//
		let (sink_a, stream_a) = listen_tcp_stream( "127.0.0.1:20001" ).await;

		// Create mailbox for peer
		//
		let mb_peer  : Inbox<MyPeer> = Inbox::new()                  ;
		let peer_addr                = Addr ::new( mb_peer.sender() );

		// create peer with stream/sink
		//
		let mut peer = Peer::new( peer_addr.clone(), stream_a.compat(), sink_a.sink_compat() ).expect( "spawn peer" );

		// Create recipients
		//
		let show = remotes::Services::recipient::<Show>( peer_addr );

		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Parallel{ sum: box show } ).expect( "spawn actor mailbox" );

		// register Sum with peer as handler for Add and Show
		//
		let mut sm = parallel::Services::new();
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );

		sm.register_with_peer( &mut peer );

		mb_peer.start( peer ).expect( "Failed to start mailbox of Peer" );
	};


	let peerb = async
	{
		let (sink_b, stream_b) = connect_return_stream( "127.0.0.1:20001" ).await;

		// Create mailbox for peer
		//
		let     mb_peer  : Inbox<MyPeer> = Inbox::new()                  ;
		let mut peer_addr                = Addr ::new( mb_peer.sender() );

		// create peer with stream/sink
		//
		let mut peer = Peer::new( peer_addr.clone(), stream_b.compat(), sink_b.sink_compat() ).expect( "spawn peer" );

		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(19) ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );

		sm.register_with_peer( &mut peer );

		mb_peer.start( peer ).expect( "Failed to start mailbox of Peer" );


		// Create recipients
		//
		let mut show = parallel::Services::recipient::<Show>( peer_addr.clone() );

		let resp = show.call( Show ).await.expect( "Call failed" );
		assert_eq!( 19, resp );

		// dbg!( resp );

		peer_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );
	};


	rt::spawn( peera  ).expect( "Spawn peera"  );
	rt::spawn( peerb  ).expect( "Spawn peerb"  );

	rt::run();
}




// Test calling a remote service after the connection has closed.
//
#[test]
//
fn call_after_close_connection()
{
	let nodea = async
	{
		// drop as soon as there is a connection
		//
		let _ = listen_tcp_stream( "127.0.0.1:20002" ).await;
	};


	let nodeb = async
	{
		let (peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20002" ).await;

		// Call the service and receive the response
		//
		let mut add = remotes::Services::recipient::<Add>( peera.clone() );

		assert_eq!( PeerEvent::ClosedByRemote,  peera_evts.next().await.unwrap() );

		match  add.call( Add(5) ).await
		{
			Ok (_) => unreachable!(),
			Err(e) =>
			{
				match e.kind()
				{
					ThesErrKind::MailboxClosed{..} => assert!( true )                        ,
					_                              => panic!( "wrong error: {:?}", e.kind() ),
				}
			}
		}
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}

