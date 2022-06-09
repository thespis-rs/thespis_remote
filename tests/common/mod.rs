#![ allow( dead_code ) ]

pub mod actors;


pub mod import
{
	pub use
	{
		async_progress  :: { Progress } ,
		async_executors :: { *                                 } ,
		futures_ringbuf :: { Endpoint                          } ,
		thespis         :: { *                                 } ,
		thespis_impl    :: { *                                 } ,
		thespis_remote  :: { * } ,
		tracing         :: { *                                 } ,
		pharos          :: { Observable, ObserveConfig, Events } ,

		std::
		{
			io           :: Write      ,
			net          :: SocketAddr ,
			convert      :: TryFrom    ,
			future       :: Future     ,
			pin          :: Pin        ,
			sync         :: Arc        ,
			time         :: Duration   ,
			sync::atomic :: { AtomicUsize, Ordering::Relaxed } ,
		},

		futures::
		{
			channel :: { mpsc::UnboundedSender                                                   } ,
			io      :: { AsyncWriteExt                                                           } ,
			compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink, SplitStream                                       } ,
			future  :: { FutureExt, join, join3, RemoteHandle                                    } ,
			task    :: { SpawnExt, LocalSpawnExt, Spawn                                          } ,
			executor:: { block_on, LocalPool, ThreadPool                                         } ,
		},

		pretty_assertions   :: { assert_eq, assert_ne } ,
		tokio               :: { sync::mpsc           } ,
	};
}


    use import::*;
pub use actors::*;



pub fn add_show_sum() -> remotes::Services
{
	// Create mailbox for our handler
	//
	let addr_handler = Addr::builder().spawn( Sum(0), &AsyncStd ).expect( "spawn actor mailbox" );

	// Create a service map
	//
	let mut sm = remotes::Services::new();

	// Register our handlers
	//
	sm.register_handler::<Add >( addr_handler.clone_box() );
	sm.register_handler::<Show>( addr_handler.clone_box() );

	sm
}


pub async fn peer_listen
(
	socket: Endpoint                                              ,
	sm    : Arc<impl ServiceMap + Send + Sync + 'static>          ,
	exec  : impl Spawn + SpawnHandle<MailboxEnd<Peer>> + PeerExec ,
	name  : &str                                                  ,
)

	-> (WeakAddr<Peer>, Events<PeerEvent>, JoinHandle< MailboxEnd<Peer> >)
{
	// create peer
	//
	let delay = Some( Duration::from_millis(10) );
	let (mut peer, peer_mb, peer_addr) = CborWF::create_peer( name, socket, 1024, 1024, Arc::new( exec.clone() ), None, delay ).expect( "spawn peer" );

	let peer_evts = peer.observe( ObserveConfig::default() ).await.expect( "pharos not closed" );

	// register service map with peer
	//
	peer.register_services( sm );

	let handle = exec.spawn_handle( peer_mb.start(peer) ).expect( "start mailbox of Peer" );

	(peer_addr, peer_evts, handle)
}




pub async fn peer_connect
(
	socket: Endpoint                                   ,
	exec  : impl Spawn + SpawnHandle<MailboxEnd<Peer>> + PeerExec ,
	name  : &str                                       ,
)
	-> (WeakAddr<Peer>, Events<PeerEvent>)

{
	// create peer with stream/sink + service map
	//
	let delay = Some( Duration::from_millis(10) );

	let (mut peer, peer_mb, peer_addr) = CborWF::create_peer( name, socket, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).await.expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( async{ peer_mb.start(peer).await; } ).expect( "start mailbox of Peer" );

	(peer_addr, evts)
}


pub async fn provider
(
	name: Option<Arc<str>>,
	exec  : impl Spawn + SpawnHandle<MailboxEnd<Peer>> + PeerExec ,
)
	-> (Endpoint, JoinHandle< MailboxEnd<Peer> >)

{
	let name = name.map( |n| n.to_string() ).unwrap_or_else( || "unnamed".to_string() );
	// Create mailbox for our handler
	//
	debug!( "start mailbox for Sum handler in provider: {}", name );
	let addr_handler = Addr::builder().spawn( Sum(0), &exec ).expect( "spawn actor mailbox" );


	// register Sum with peer as handler for Add and Show
	//
	let mut sm = remotes::Services::new();

	sm.register_handler::<Add >( addr_handler.clone_box() );
	sm.register_handler::<Show>( addr_handler.clone_box() );


	// get a framed connection
	//
	let (ab, ba) = Endpoint::pair( 128, 128 );

	debug!( "start mailbox for provider" );
	let (_, _peer_evts, handle) = peer_listen( ab, Arc::new( sm ), exec, "provider" ).await;

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
	exec      : impl Spawn + SpawnHandle<MailboxEnd<Peer>> + PeerExec  ,
)
{
	debug!( "start mailbox for relay_to_provider" );

	let (mut provider_addr, _provider_evts) = peer_connect( connect, exec.clone(), "relay_to_provider" ).await;
	let provider_addr2                      = provider_addr.clone();
	let ex1                                 = exec.clone();

	// Relay part ---------------------

	let relay = async move
	{
		// create peer with stream/sink + service map
		//
		let delay = Some( Duration::from_millis(10) );

		let (mut peer, peer_mb, _) = CborWF::create_peer( "relay_to_consumer", listen, 1024, 1024, ex1, None, delay ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();

		let handler: Box<dyn Relay<CborWF>> = Box::new( provider_addr2 );

		let mut relayed = vec![ add ];

		if relay_show
		{
			relayed.push( show );
		}

		let rm = Arc::new( RelayMap::new( handler.into(), relayed ) );
		peer.register_services( rm );

		debug!( "start mailbox for relay_to_consumer" );
		peer_mb.start( peer ).await;
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
	exec      : impl Spawn + SpawnHandle<MailboxEnd<Peer>> + PeerExec,
)
{
	debug!( "start mailbox for relay_to_provider" );

	let mut providers: Vec<WeakAddr<Peer>> = Vec::new();

	for (idx, endpoint) in connect.into_iter().enumerate()
	{
		let name = format!( "relay_to_provider{}", idx );
		let (provider_addr, _provider_evts) = peer_connect( endpoint, exec.clone(), &name ).await;

		providers.push( provider_addr );
	}

	let providers2 = providers.clone();
	let ex1        = exec.clone();

	// Relay part ---------------------

	let relay = async move
	{
		// create peer with stream/sink + service map
		//
		let delay = Some( Duration::from_millis(10) );

		let (mut peer, peer_mb, _) = CborWF::create_peer( "relay_to_consumer", listen, 1024, 1024, ex1, None, delay ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();


		let handler = Box::new( move |_: &ServiceID| -> Box<dyn Relay<CborWF>>
		{
			static IDX: AtomicUsize = AtomicUsize::new( 0 );

			let   i  = IDX.fetch_add( 1, Relaxed );
			let addr = &providers2[ i % providers2.len() ];

			Box::new( addr.clone() )

		});


		let mut relayed = vec![ add ];

		if relay_show
		{
			relayed.push( show );
		}

		let rm = Arc::new( RelayMap::new( ServiceHandler::Closure( handler ), relayed ) );

		peer.register_services( rm );

		debug!( "start mailbox for relay_to_consumer" );
		peer_mb.start( peer ).await;
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
	namespace  : remotes        ;
	wire_format: CborWF         ;
	services   : Add, Sub, Show ;
);

