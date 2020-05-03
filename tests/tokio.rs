// Tests:
//
// - ✔ basic remote funcionality: intertwined sends and calls.
// - ✔ correct async behavior: verify that a peer can continue to send/receive while waiting for the response to a call.
// - ✔ call a remote service after the connection has closed: verify peer event and error kind.
//
#![ cfg( feature = "tokio_codec" ) ]

mod common;

use
{
	common::{ remotes, Add, Sum, Show, import::{ *, assert_eq } } ,
	futures_ringbuf::TokioEndpoint ,
	tokio_util::codec::{ Framed } ,
	tokio::io::{ AsyncWriteExt } ,
};



pub fn peer_listen
(
	socket: TokioEndpoint                                ,
	sm    : Arc<impl ServiceMap + Send + Sync + 'static> ,
	exec  : impl Spawn + SpawnHandle< Result<Response, PeerErr> > + Clone + Send + Sync + 'static ,
	name  : &'static str                                 ,
)

	-> (Addr<Peer>, Events<PeerEvent>)
{
	// Create mailbox for peer
	//
	let (peer_addr, peer_mb) = Addr::builder().name( name.into() ).build();

	// create peer with stream/sink
	//
	let mut peer = Peer::from_tokio_async_read( peer_addr.clone(), socket, 1024, exec.clone(), None ).expect( "spawn peer" );

	let peer_evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	// register service map with peer
	//
	peer.register_services( sm );

	exec.spawn( async { peer_mb.start(peer).await; } ).expect( "start mailbox of Peer" );

	(peer_addr, peer_evts)
}


pub async fn peer_connect
(
	socket: TokioEndpoint,
	exec: impl Spawn + SpawnHandle< Result<Response, PeerErr> > + Clone + Send + Sync + 'static,
	name: &'static str
)
	-> (Addr<Peer>, Events<PeerEvent>)
{
	// Create mailbox for peer
	//
	let (addr, mb) = Addr::builder().name( name.into() ).build();

	// create peer with stream/sink + service map
	//
	let mut peer = Peer::from_tokio_async_read( addr.clone(), socket, 1024, exec.clone(), None ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( async { mb.start(peer).await; } ).expect( "start mailbox of Peer" );

	(addr, evts)
}



// Test basic remote funcionality. Test intertwined sends and calls.
//
#[test]
//
fn remote()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = TokioEndpoint::pair( 64, 64 );

	let exec = Arc::new( ThreadPool::new().expect( "create threadpool" ) );
	let ex1  = exec.clone();


	let peera = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::builder().start( Sum(0), &ex1 ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let mut sm = remotes::Services::new();
		// Register our handlers
		//
		sm.register_handler::<Add >( addr_handler.clone_box() );
		sm.register_handler::<Show>( addr_handler.clone_box() );

		// get a framed connection
		//
		let _ = peer_listen( server, Arc::new( sm ), ex1.clone(), "peera" );

		trace!( "end of peera" );
	};


	let peerb = async move
	{
		let (mut peera, _)  = peer_connect( client, exec, "peer_b_to_peera" ).await;

		// Call the service and receive the response
		//
		let mut addr  = remotes::RemoteAddr::new( peera.clone() );

		assert_eq!( addr.call( Add(5) ).await, Ok(()) );

		addr.send( Add(5) ).await.expect( "Send failed" );

		let resp = addr.call( Show ).await.expect( "Call failed" );
		assert_eq!( 10, resp );

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	block_on( join( peera, peerb ) );
}




#[ derive( Actor ) ]
//
pub struct Parallel
{
	pub sum: Box< dyn Address<Show, Error=PeerErr> >,
}


impl Handler< Show > for Parallel
{
	fn handle( &mut self, _: Show ) -> Return<i64> { async move
	{
		self.sum.call( Show ).await.expect( "call sum" )

	}.boxed() }
}


service_map!
(
	namespace     : parallel ;
	services      : Show     ;
);




// Test correct async behavior. Verify that a peer can continue to
// send/receive while waiting for the response to a call.
//
#[test]
//
fn parallel()
{
	let (server, client) = TokioEndpoint::pair( 64, 64 );

	let exec = Arc::new( ThreadPool::new().expect( "create threadpool" ) );
	let ex1  = exec.clone();

	let peera = async move
	{
		let codec: ThesCodec = ThesCodec::new(1024);

		// get a framed connection
		//
		let (sink_a, stream_a) = Framed::new( server, codec ).split();

		// Create mailbox for peer
		//
		let (peer_addr, peer_mb) = Addr::builder().name( "peer_a".into() ).build();

		// create peer with stream/sink
		//
		let mut peer = Peer::new( peer_addr.clone(), stream_a, sink_a, ex1.clone(), None ).expect( "spawn peer" );

		// Create recipients
		//
		let addr = remotes::RemoteAddr::new( peer_addr );

		// Create mailbox for our handler
		//
		let addr_handler = Addr::builder().start( Parallel{ sum: Box::new( addr ) }, &ex1 ).expect( "spawn actor mailbox" );

		// register Sum with peer as handler for Add and Show
		//
		let mut sm = parallel::Services::new();
		sm.register_handler::<Show>( addr_handler.clone_box() );

		peer.register_services( Arc::new( sm ) );

		peer_mb.spawn( peer, &ex1 ).expect( "Failed to start mailbox of Peer" );
	};


	let peerb = async move
	{
		// Create mailbox for peer
		//
		let (mut peer_addr, peer_mb) = Addr::builder().name( "peer_b".into() ).build();

		// create peer with stream/sink
		//
		let mut peer = Peer::from_tokio_async_read( peer_addr.clone(), client, 1024, exec.clone(), None ).expect( "spawn peer" );

		// Create mailbox for our handler
		//
		let addr_handler = Addr::builder().start( Sum(19), &exec ).expect( "spawn actor mailbox" );


		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();
		sm.register_handler::<Show>( addr_handler.clone_box() );

		peer.register_services( Arc::new( sm ) );

		peer_mb.spawn( peer, &exec ).expect( "Failed to start mailbox of Peer" );


		// Create recipients
		//
		let mut remote = parallel::RemoteAddr::new( peer_addr.clone() );

		let resp = remote.call( Show ).await.expect( "Call failed" );
		assert_eq!( 19, resp );

		// dbg!( resp );

		peer_addr.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};

	block_on( join( peera, peerb ) );
}




// Test calling a remote service after the connection has closed.
//
#[test]
//
fn call_after_close_connection()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (mut server, client) = TokioEndpoint::pair( 64, 64 );

	let exec = Arc::new( ThreadPool::new().expect( "create threadpool" ) );

	let nodea = async move
	{
		server.shutdown().await.expect( "close connection" );
	};


	let nodeb = async move
	{
		let (peera, mut peera_evts) = peer_connect( client, exec, "nodeb_to_node_a" ).await;

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
					_                                   => panic!( "wrong error: {:?}", e ) ,
				}
			}
		}
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	block_on( join( nodea, nodeb ) );
}

