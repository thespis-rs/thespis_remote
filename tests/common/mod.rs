#![ allow( dead_code ) ]

pub mod actors;


pub mod import
{
	pub use
	{
		async_executors :: { *                                 } ,
		futures_ringbuf :: { Endpoint                          } ,
		thespis         :: { *                                 } ,
		thespis_impl    :: { *                                 } ,
		thespis_remote  :: { *, service_map, peer              } ,
		log             :: { *                                 } ,
		bytes           :: { Bytes, BytesMut                   } ,
		pharos          :: { Observable, ObserveConfig, Events } ,

		std::
		{
			net          :: SocketAddr ,
			convert      :: TryFrom    ,
			future       :: Future     ,
			pin          :: Pin        ,
			sync         :: Arc        ,
			sync::atomic :: { AtomicUsize, Ordering::Relaxed } ,
		},

		futures::
		{
			io      :: { AsyncWriteExt                                                           } ,
			compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink, SplitStream                                       } ,
			future  :: { FutureExt, join, join3, RemoteHandle                                    } ,
			task    :: { SpawnExt, LocalSpawnExt, Spawn                                          } ,
			executor:: { block_on, LocalPool, ThreadPool                                         } ,
		},

		pretty_assertions :: { assert_eq, assert_ne } ,
		assert_matches    :: { assert_matches       } ,
		tokio             :: { sync::mpsc           } ,
		async_chanx       :: { TokioSender          } ,
	};
}


    use import::*;
pub use actors::*;


pub async fn peer_listen
(
	socket: Endpoint                                                     ,
	sm    : impl ServiceMap + Send + 'static                             ,
	exec  : impl Spawn + SpawnHandle<()> + Clone + Send + Sync + 'static ,
	name  : &str                                                         ,
)

	-> (Addr<Peer>, Events<PeerEvent>, JoinHandle<()>)
{
	// Create mailbox for peer
	//
	let (peer_addr, peer_mb) = Addr::builder().name( name.into() ).build()                            ;

	// create peer with stream/sink
	//
	let mut peer = Peer::from_async_read( peer_addr.clone(), socket, 1024, Arc::new( exec.clone() ), None ).expect( "spawn peer" );

	let peer_evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	// register service map with peer
	//
	peer.register_services( Box::new(sm) ).await.expect( "register services" );

	let handle = exec.spawn_handle( peer_mb.start_fut(peer) ).expect( "start mailbox of Peer" );

	(peer_addr, peer_evts, handle)
}




pub fn peer_connect
(
	socket: Endpoint                                   ,
	exec  : impl Spawn + Clone + Send + Sync + 'static ,
	name  : &str                                       ,
)
	-> (Addr<Peer>, Events<PeerEvent>)

{
	// Create mailbox for peer
	//
	let (tx, rx)    = mpsc::channel( 16 )                                                             ;
	let mb_peer     = Inbox::new( Some( name.into() ), Box::new( rx ) )                               ;
	let tx          = Box::new( TokioSender::new( tx ).sink_map_err( |e| Box::new(e) as SinkError ) ) ;
	let peer_addr   = Addr::new( mb_peer.id(), mb_peer.name(), tx )                                   ;


	// create peer with stream/sink + service map
	//
	let mut peer = Peer::from_async_read( peer_addr.clone(), socket, 1024, exec.clone(), None ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( mb_peer.start_fut(peer) ).expect( "start mailbox of Peer" );

	(peer_addr, evts)
}


pub async fn provider
(
	name: Option<Arc<str>>,
	exec  : impl Spawn + SpawnHandle<()> + Clone + Send + Sync + 'static ,
)
	-> (Endpoint, JoinHandle<()>)

{
	let name = name.map( |n| n.to_string() ).unwrap_or( "unnamed".to_string() );
	// Create mailbox for our handler
	//
	debug!( "start mailbox for Sum handler in provider: {}", name );
	let addr_handler = Addr::builder().start( Sum(0), &exec ).expect( "spawn actor mailbox" );


	// register Sum with peer as handler for Add and Show
	//
	let mut sm = remotes::Services::new();

	sm.register_handler::<Add >( addr_handler.clone_box() );
	sm.register_handler::<Show>( addr_handler.clone_box() );

	let sm_addr = Addr::builder().start( sm, &exec ).expect( "spawn sm mailbox" );


	// get a framed connection
	//
	let (ab, ba) = Endpoint::pair( 128, 128 );

	debug!( "start mailbox for provider" );
	let (peer_addr, _peer_evts, handle) = peer_listen( ab, sm_addr, exec, "provider" ).await;

	drop( peer_addr );
	trace!( "End of provider" );

	(ba, handle)
}



// Helper method to create relays
//
pub async fn relay
(
	connect   : Endpoint                                 ,
	listen    : Endpoint                                 ,
	next      : Pin<Box< dyn Future<Output=()> + Send >> ,
	relay_show: bool                                     ,
	exec      : Arc<dyn Spawn + Send + Sync + 'static >  ,
)
{
	debug!( "start mailbox for relay_to_provider" );

	let (mut provider_addr, _provider_evts) = peer_connect( connect, exec.clone(), "relay_to_provider" );
	let provider_addr2                      = provider_addr.clone();
	let ex1                                 = exec.clone();
	let ex2                                 = exec.clone();

	// Relay part ---------------------

	let relay = async move
	{
		// Create mailbox for peer
		//
		let (peer_addr, peer_mb) = Addr::builder().name( "relay_to_consumer".into() ).build();

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::from_async_read( peer_addr, listen, 1024, ex1, None ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();

		let handler: Box<dyn Relay> = Box::new( provider_addr2 );

		let mut relayed = vec![ add.clone() ];

		if relay_show
		{
			relayed.push( show.clone() );
		}

		let rm = RelayMap::new( handler.into(), relayed );
		let rm_addr = Addr::builder().start( rm, &ex2 ).expect( "spawn relay map" );

		peer.register_services( Box::new( rm_addr ) ).await.expect( "register services" );

		debug!( "start mailbox for relay_to_consumer" );
		peer_mb.start_fut( peer ).await;
		warn!( "relay async block finished" );
	};


	let (relay_fut, relay_outcome) = relay.remote_handle();
	exec.spawn( relay_fut ).expect( "failed to spawn server" );

	// we need to spawn this after this relay, otherwise this relay is not listening yet when we try to connect.
	//
	exec.spawn( next ).expect( "Spawn next" );


	// If the consumer closes the connection, close our connection to provider.
	//
	relay_outcome.await;
	warn!( "relay finished, closing connection" );

	provider_addr.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to provider" );
}



// Helper method to create relays
//
pub async fn relay_closure
(
	connect   : Vec<Endpoint>                            ,
	listen    : Endpoint                                 ,
	next      : Pin<Box< dyn Future<Output=()> + Send >> ,
	relay_show: bool                                     ,
	exec      : Arc<dyn Spawn + Send + Sync + 'static >  ,
)
{
	debug!( "start mailbox for relay_to_provider" );

	let mut providers: Vec<Addr<Peer>> = Vec::new();

	for (idx, endpoint) in connect.into_iter().enumerate()
	{
		let name = format!( "relay_to_provider{}", idx );
		let (provider_addr, _provider_evts) = peer_connect( endpoint, exec.clone(), &name );

		providers.push( provider_addr );
	}

	let providers2 = providers.clone();
	let ex1        = exec.clone();
	let ex2        = exec.clone();

	// Relay part ---------------------

	let relay = async move
	{
		// Create mailbox for peer
		//
		let (peer_addr, peer_mb) = Addr::builder().name( "relay_to_consumer".into() ).build();


		// create peer with stream/sink + service map
		//
		let mut peer = Peer::from_async_read( peer_addr, listen, 1024, ex1, None ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();


		let handler = Box::new( move |_: &ServiceID| -> Box<dyn Relay>
		{
			static IDX: AtomicUsize = AtomicUsize::new( 0 );

			let   i  = IDX.fetch_add( 1, Relaxed );
			let addr = &providers2[ i % providers2.len() ];

			Box::new( addr.clone() )

		});


		let mut relayed = vec![ add.clone() ];

		if relay_show
		{
			relayed.push( show.clone() );
		}

		let rm = RelayMap::new( ServiceHandler::Closure( handler ), relayed );
		let rm_addr = Addr::builder().start( rm, &ex2 ).expect( "spawn relay map" );

		peer.register_services( Box::new( rm_addr ) ).await.expect( "register services" );


		debug!( "start mailbox for relay_to_consumer" );
		peer_mb.start_fut( peer ).await;
		warn!( "relay async block finished" );
	};


	let (relay_fut, relay_outcome) = relay.remote_handle();
	exec.spawn( relay_fut ).expect( "failed to spawn server" );

	// we need to spawn this after this relay, otherwise this relay is not listening yet when we try to connect.
	//
	exec.spawn( next ).expect( "Spawn next" );


	// If the consumer closes the connection, close our connection to provider.
	//
	relay_outcome.await;
	warn!( "relay finished, closing connection" );

	for mut addr in providers
	{
		addr.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to provider" );
	}
}





service_map!
(
	namespace: remotes        ;
	services : Add, Sub, Show ;
);

