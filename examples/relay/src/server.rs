mod common;

use common::{ import::*, * };


#[ async_std::main ]
//
async fn main()
{
	flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	// Create mailbox for our handler
	//
	debug!( "start mailbox for handler" );
	let addr_handler = Addr::builder().start( Sum(0), &AsyncStd ).expect( "spawn actor mailbox" );

	// Get a TCP connection.
	//
	let     listener = TcpListener::bind( SERVER ).await.expect( "server listen" );
	let mut incoming = listener.incoming();

	let socket = incoming.next().await.expect( "one connection" ).expect( "valid tcp" );

	// Create mailbox for peer
	//
	let (peer_addr, peer_mb) = Addr::builder().name( "server".into() ).build();

	// Create peer with stream/sink
	//
	let mut peer = Peer::from_async_read( peer_addr, socket, 1024, Arc::new( AsyncStd ), None ).expect( "create peer" );


	// Register Sum with peer as handler for Add and Show
	//
	let mut sm = remotes::Services::new();

	sm.register_handler::<Add >( Box::new( Receiver::new( addr_handler.clone_box() ) ) );
	sm.register_handler::<Show>( Box::new( Receiver::new( addr_handler.clone_box() ) ) );


	// Register service map with peer
	//
	peer.register_services( Arc::new( sm ) );


	// Start the peer actor
	//
	let peer_handle = peer_mb.start_handle( peer, &AsyncStd ).expect( "start mailbox of Peer" );


	// Wait for the connection to close, which will automatically stop the peer.
	//
	peer_handle.await;

	trace!( "End of server" );
}
