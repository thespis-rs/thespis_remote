#![ allow( dead_code ) ]

pub mod actors;


pub mod import
{
	pub use
	{
		async_executors     :: { LocalPool, AsyncStd, ThreadPool, JoinHandle, SpawnHandle, LocalSpawnHandle } ,
		futures_ringbuf     :: { Endpoint                                                                   } ,
		thespis             :: { *                                                                          } ,
		thespis_impl        :: { *                                                                          } ,
		thespis_remote_impl :: { *, service_map, peer                                                       } ,
		log                 :: { *                                                                          } ,
		bytes               :: { Bytes, BytesMut                                                            } ,
		pharos              :: { Observable, ObserveConfig, Events                                          } ,

		std::
		{
			net     :: SocketAddr ,
			convert :: TryFrom    ,
			future  :: Future     ,
			pin     :: Pin        ,
			sync    :: Arc        ,
		},

		futures::
		{
			channel :: { mpsc                                                                    } ,
			io      :: { AsyncWriteExt                                                           } ,
			compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink, SplitStream                                       } ,
			future  :: { FutureExt, join, RemoteHandle                                           } ,
			task    :: { SpawnExt, LocalSpawnExt, Spawn                                          } ,
			executor:: { block_on                                                                } ,
		},

		pretty_assertions :: { assert_eq, assert_ne } ,
		assert_matches    :: { assert_matches       } ,

	};
}


    use import::*;
pub use actors::*;


pub fn peer_listen( socket: Endpoint, sm: Arc<impl ServiceMap + Send + Sync + 'static>, exec: impl Spawn + Clone + Send + Sync + 'static, name: &str )

	-> (Addr<Peer>, Events<PeerEvent>, RemoteHandle<()>)
{
	// Create mailbox for peer
	//
	let mb_peer  : Inbox<Peer> = Inbox::new( Some( name.into() ) );
	let peer_addr              = Addr ::new( mb_peer.sender() );

	// create peer with stream/sink
	//
	let mut peer = Peer::from_async_read( peer_addr.clone(), socket, 1024, exec.clone() ).expect( "spawn peer" );

	let peer_evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	// register service map with peer
	//
	peer.register_services( sm );

	let (fut, handle) = mb_peer.start_fut(peer).remote_handle();

	exec.spawn( fut ).expect( "start mailbox of Peer" );

	(peer_addr, peer_evts, handle)
}




pub fn peer_connect( socket: Endpoint, exec: impl Spawn + Clone + Send + Sync + 'static, name: &str ) -> (Addr<Peer>, Events<PeerEvent>)
{
	// Create mailbox for peer
	//
	let mb  : Inbox<Peer> = Inbox::new( Some( name.into() ) );
	let addr              = Addr ::new( mb.sender() );

	// create peer with stream/sink + service map
	//
	let mut peer = Peer::from_async_read( addr.clone(), socket, 1024, exec.clone() ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( mb.start_fut(peer) ).expect( "start mailbox of Peer" );

	(addr, evts)
}




// Helper method to create relays
//
pub async fn relay
(
	connect   : Endpoint                                   ,
	listen    : Endpoint                                   ,
	next      : Pin<Box< dyn Future<Output=()> + Send >>   ,
	relay_show: bool                                       ,
	exec      : impl Spawn + Clone + Send + Sync + 'static ,
)
{
	debug!( "start mailbox for relay_to_provider" );

	let (mut provider_addr, _provider_evts) = peer_connect( connect, exec.clone(), "relay_to_provider" );
	let provider_addr2                      = provider_addr.clone();
	let ex1                                 = exec.clone();

	// Relay part ---------------------

	let relay = async move
	{
		// Create mailbox for peer
		//
		let mb_peer  : Inbox<Peer> = Inbox::new( Some( "relay_to_consumer".into() ) );
		let peer_addr              = Addr ::new( mb_peer.sender()                   );

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::from_async_read( peer_addr, listen, 1024, ex1 ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();

		let rm   = Arc::new( RelayMap::new() );

		let h1: Box<dyn Relay> = Box::new( provider_addr2.clone() );
		let h2: Box<dyn Relay> = Box::new( provider_addr2         );

		rm.register_handler( add.clone(), ServiceHandler::from( h1 ) );

		if relay_show
		{
			rm.register_handler( show.clone(), ServiceHandler::from( h2 ) );
		}


		peer.register_services( rm );

		debug!( "start mailbox for relay_to_consumer" );
		mb_peer.start_fut( peer ).await;
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

	provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to provider" );
}





service_map!
(
	namespace: remotes   ;
	services : Add, Show ;
);

