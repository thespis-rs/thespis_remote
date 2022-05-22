// Tests:
//
// ✔ basic remote funcionality: intertwined sends and calls.
// ✔ correct async behavior: verify that a peer can continue to send/receive while waiting for the response to a call.
// ✔ call a remote service after the connection has closed: verify peer event and error kind.
//
mod common;

use common::*                       ;
use common::import::{ *, assert_eq };


// Test basic remote funcionality. Test intertwined sends and calls.
//
#[async_std::test]
//
async fn basic_remote()
{
	// flexi_logger::Logger::with_str( "info, thespis_remote=trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );


	let peera = async move
	{
		// get a framed connection
		//
		let (_, _, handle) = peer_listen( server, Arc::new( add_show_sum() ), AsyncStd, "peera" ).await;

		handle.await;

		trace!( "end of peera" );
	};


	let peerb = async move
	{
		let (mut peera, _)  = peer_connect( client, AsyncStd, "peer_b_to_peera" ).await;

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( peera.clone() );

		// The receiving end doesn't guarantee order of processing with send, so we only
		// use call here.
		//
		assert_eq!( Ok(()), addr.call( Add(5) ).await );
		assert_eq!( Ok(()), addr.call( Add(5) ).await );
		assert_eq!( Ok(10), addr.call( Show   ).await );

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	join( peera, peerb ).await;
}




#[ derive( Actor ) ]
//
pub struct Parallel
{
	pub sum: Box< dyn Address<Show, Error=PeerErr> >,
}


impl Handler< Show > for Parallel
{
	fn handle( &mut self, _: Show ) -> Return<i64> { Box::pin( async move
	{
		self.sum.call( Show ).await.expect( "call sum" )
	})}
}


service_map!
(
	namespace  : parallel ;
	wire_format: CborWF   ;
	services   : Show     ;
);




// Test correct async behavior. Verify that a peer can continue to
// send/receive while waiting for the response to a call.
//
// TODO: the spawning is now in service map and relaymap, so we actually
// should also test that they process requests concurrently.
//
#[async_std::test]
//
async fn parallel()
{
	let (server, client) = Endpoint::pair( 64, 64 );

	let peera = async move
	{
		// Create mailbox for peer
		//
		let (peer_addr, peer_mb) = Addr::builder().name( "peer_a" ).build();

		// create peer with stream/sink
		//
		let mut peer = CborWF::create_peer( peer_addr.clone(), server, 1024, 1024, AsyncStd, None, None ).expect( "spawn peer" );

		// Create recipients
		//
		let addr = remotes::RemoteAddr::new( peer_addr );

		// Create mailbox for our handler
		//
		let addr_handler = Addr::builder().spawn( Parallel{ sum: Box::new( addr ) }, &AsyncStd ).expect( "spawn actor mailbox" );

		// register Sum with peer as handler for Add and Show
		//
		let mut sm = parallel::Services::new();
		sm.register_handler::<Show>( addr_handler.clone_box() );

		peer.register_services( Arc::new( sm ) );

		AsyncStd.spawn( peer_mb.start(peer).map(|_|()) ).expect( "Failed to start mailbox of Peer" );
	};


	let peerb = async move
	{
		// Create mailbox for peer
		//
		let (mut peer_addr, peer_mb) = Addr::builder().name( "peer_b" ).build();

		// create peer with stream/sink
		//
		let mut peer = CborWF::create_peer( peer_addr.clone(), client, 1024, 1024, AsyncStd, None, None ).expect( "spawn peer" );

		// Create mailbox for our handler
		//
		let addr_handler = Addr::builder().spawn( Sum(19), &AsyncStd ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();
		sm.register_handler::<Show>( addr_handler.clone_box() );

		peer.register_services( Arc::new( sm ) );


		AsyncStd.spawn( peer_mb.start(peer).map(|_|()) ).expect( "Failed to start mailbox of Peer" );

		// Create recipients
		//
		let mut remote = parallel::RemoteAddr::new( peer_addr.clone() );

		let resp = remote.call( Show ).await.expect( "Call failed" );
		assert_eq!( 19, resp );

		// dbg!( resp );

		peer_addr.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};

	join( peera, peerb ).await;
}




// Test calling a remote service after the connection has closed.
//
#[async_std::test]
//
async fn call_after_close_connection()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (mut server, client) = Endpoint::pair( 64, 64 );


	let nodea = async move
	{
		server.close().await.expect( "close connection" );
	};


	let nodeb = async move
	{
		let (peera, mut peera_evts) = peer_connect( client, AsyncStd, "nodeb_to_node_a" ).await;

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( peera.clone() );

		assert_eq!( PeerEvent::ClosedByRemote, peera_evts.next().await.unwrap() );


		match addr.call( Add(5) ).await
		{
			Ok (_) => unreachable!(),
			Err(e) =>
			{
				match e
				{
					PeerErr::ConnectionClosed{..} => {}
					_                             => panic!( "wrong error: {:?}", e ) ,
				}
			}
		}
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	join( nodea, nodeb ).await;
}

