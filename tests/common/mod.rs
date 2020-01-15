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
			future  :: { FutureExt, join                                                         } ,
			task    :: { SpawnExt, LocalSpawnExt, Spawn                                          } ,
			executor:: { block_on                                                                } ,
		},

		pretty_assertions :: { assert_eq, assert_ne } ,
		assert_matches    :: { assert_matches       } ,

	};
}


    use import::*;
pub use actors::*;


pub fn peer_listen( socket: Endpoint, sm: Arc<impl ServiceMap + Send + Sync + 'static>, exec: impl Spawn + Clone + Send + Sync + 'static, name: &'static str ) -> (Addr<Peer>, Events<PeerEvent>)
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

	exec.spawn( mb_peer.start_fut(peer) ).expect( "start mailbox of Peer" );

	(peer_addr, peer_evts)
}




pub async fn peer_connect( socket: Endpoint, exec: impl Spawn + Clone + Send + Sync + 'static, name: &'static str ) -> (Addr<Peer>, Events<PeerEvent>)
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


service_map!
(
	namespace: remotes   ;
	services : Add, Show ;
);

