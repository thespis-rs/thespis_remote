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

		pretty_assertions::{ assert_eq, assert_ne }
	};
}

#[ cfg( feature = "tokio_codec" ) ]
//
pub(crate) use
{
	tokio::
	{
		prelude :: { Stream as TokStream, stream::{ SplitStream as TokSplitStream, SplitSink as TokSplitSink } } ,
	},
};


#[ cfg( feature = "futures_codec" ) ]
//
pub(crate) use
{
	futures_codec_crate :: { Framed } ,
};



    use import::*;
pub use actors::*;

pub type TheSink = SplitSink< Framed< Endpoint, ThesCodec >, WireFormat> ;


pub fn peer_listen( socket: Endpoint, sm: Arc<impl ServiceMap + Send + Sync + 'static>, exec: impl Spawn + Clone + Send + 'static ) -> (Addr<Peer>, Events<PeerEvent>)
{
	let codec = ThesCodec::new(1024);

	let (sink, stream) = Framed::new( socket, codec ).split();

	// Create mailbox for peer
	//
	let mb_peer  : Inbox<Peer> = Inbox::default()              ;
	let peer_addr              = Addr ::new( mb_peer.sender() );

	// create peer with stream/sink
	//
	let mut peer = Peer::new( peer_addr.clone(), stream, sink, exec.clone() ).expect( "create peer" );

	let peer_evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	// register service map with peer
	//
	peer.register_services( sm );

	exec.spawn( mb_peer.start_fut(peer) ).expect( "start mailbox of Peer" );

	(peer_addr, peer_evts)
}




pub async fn peer_connect( socket: Endpoint, exec: impl Spawn + Clone + Send + 'static, name: &'static str ) -> (Addr<Peer>, Events<PeerEvent>)
{
	// frame the connection with codec for multiservice
	//
	let codec: ThesCodec = ThesCodec::new(1024);

	let (sink_a, stream_a) = Framed::new( socket, codec ).split();

	// Create mailbox for peer
	//
	let mb  : Inbox<Peer> = Inbox::new( name.into() );
	let addr              = Addr ::new( mb.sender() );

	// create peer with stream/sink + service map
	//
	let mut peer = Peer::new( addr.clone(), stream_a, sink_a, exec.clone() ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( mb.start_fut(peer) ).expect( "start mailbox of Peer" );

	(addr, evts)
}



pub async fn connect_return_stream( socket: Endpoint ) ->

	(SplitSink<Framed<Endpoint, ThesCodec>, WireFormat>, SplitStream<Framed<Endpoint, ThesCodec>>)

{
	// frame the connection with codec for multiservice
	//
	let codec: ThesCodec = ThesCodec::new(1024);

	Framed::new( socket, codec ).split()
}




service_map!
(
	namespace: remotes   ;
	services : Add, Show ;
);

