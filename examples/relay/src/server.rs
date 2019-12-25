mod common;

use common::{ import::*, * };


fn main()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let exec = AsyncStd::default();
	let ex1  = exec.clone();


	let server = async move
	{
		// Create mailbox for our handler
		//
		debug!( "start mailbox for handler" );
		let addr_handler = Addr::try_from( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Get a TCP connection.
		//
		let     listener = TcpListener::bind( SERVER ).await.expect( "server listen" );
		let mut incoming = listener.incoming();

		let socket = incoming.next().await.expect( "one connection" ).expect( "valid tcp" );

		// Create mailbox for peer
		//
		let mb_peer  : Inbox<Peer> = Inbox::new( "server".into()  );
		let peer_addr              = Addr ::new( mb_peer.sender() );

		// Create peer with stream/sink
		//
		let mut peer = Peer::from_async_read( peer_addr, socket, 1024 ).expect( "create peer" );


		// Register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );


		// Register service map with peer
		//
		peer.register_services( Arc::new( sm ) );


		// Start the peer actor
		//
		let peer_handle = exec.spawn_handle( mb_peer.start_fut(peer) ).expect( "start mailbox of Peer" );


		// Wait for the connection to close, which will automatically stop the peer.
		//
		peer_handle.await;

		trace!( "End of server" );
	};


	// --------------------------------------


	block_on( server );
}
