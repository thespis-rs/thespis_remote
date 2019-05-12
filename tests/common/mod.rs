#![ allow( dead_code ) ]

pub mod actors;


pub mod import
{
	pub use
	{
		thespis             :: { *                    } ,
		thespis_remote      :: { *                    } ,
		thespis_impl        :: { *, runtime::{ rt }   } ,
		thespis_impl_remote :: { *, service_map, peer } ,
		log                 :: { *                    } ,
		std                 :: { net::SocketAddr, convert::TryFrom } ,

		bytes               :: { Bytes                } ,
		failure             :: { Fail                 } ,
		pharos              :: { Observable           } ,


		futures      ::
		{
			channel :: { mpsc                                                                    } ,
			compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
			prelude :: { StreamExt                                                               } ,
			future  :: { FutureExt                                                               } ,
		},


		tokio        ::
		{
			prelude :: { Stream as TokStream, stream::{ SplitStream as TokSplitStream, SplitSink as TokSplitSink } } ,
	 		net     :: { TcpStream, TcpListener                                                                    } ,
			codec   :: { Decoder, Framed                                                                           } ,
		},

		pretty_assertions::{ assert_eq, assert_ne }
	};
}

    use import::*;
pub use actors::*;

pub type TheSink = Compat01As03Sink<TokSplitSink<Framed<TcpStream, MulServTokioCodec<MS>>>, MS> ;
pub type MS      = MultiServiceImpl<ServiceID, ConnID, Codecs>                                  ;
pub type MyPeer  = Peer<TheSink, MS>                                                            ;


pub async fn listen_tcp( socket: &str, sm: impl ServiceMap<MS> ) -> (Addr<MyPeer>, mpsc::Receiver<PeerEvent>)
{
	// create tcp server
	//
	let socket   = socket.parse::<SocketAddr>().unwrap();
	let listener = TcpListener::bind( &socket ).expect( "bind address" );

	let codec: MulServTokioCodec<MS> = MulServTokioCodec::new();

	let stream   = await!( listener.incoming().take(1).into_future().compat() )
		.expect( "find one stream" ).0
		.expect( "find one stream" );

	let (sink, stream) = codec.framed( stream ).split();

	// Create mailbox for peer
	//
	let mb_peer  : Inbox<MyPeer> = Inbox::new()                  ;
	let peer_addr                = Addr ::new( mb_peer.sender() );

	// create peer with stream/sink
	//
	let mut peer = Peer::new( peer_addr.clone(), stream.compat(), sink.sink_compat() ).expect( "create peer" );

	let peer_evts = peer.observe( 10 );

	// register service map with peer
	//
	sm.register_with_peer( &mut peer );

	mb_peer.start( peer ).expect( "Failed to start mailbox of Peer" );

	(peer_addr, peer_evts)
}



pub async fn listen_tcp_stream( socket: &str ) ->

	(TokSplitSink<Framed<TcpStream, MulServTokioCodec<MS>>>, TokSplitStream<Framed<TcpStream, MulServTokioCodec<MS>>>)

{
	// create tcp server
	//
	let socket   = socket.parse::<SocketAddr>().unwrap();
	let listener = TcpListener::bind( &socket ).expect( "bind address" );

	let codec: MulServTokioCodec<MS> = MulServTokioCodec::new();

	let stream   = await!( listener.incoming().take(1).into_future().compat() )
		.expect( "find one stream" ).0
		.expect( "find one stream" );

	codec.framed( stream ).split()
}




pub async fn connect_to_tcp( socket: &str ) -> (Addr<MyPeer>, mpsc::Receiver<PeerEvent>)
{
	// Connect to tcp server
	//
	let socket = socket.parse::<SocketAddr>().unwrap();
	let stream = await!( TcpStream::connect( &socket ).compat() ).expect( "connect address" );

	// frame the connection with codec for multiservice
	//
	let codec: MulServTokioCodec<MS> = MulServTokioCodec::new();

	let (sink_a, stream_a) = codec.framed( stream ).split();

	// Create mailbox for peer
	//
	let mb  : Inbox<MyPeer> = Inbox::new()             ;
	let addr                = Addr ::new( mb.sender() );

	// create peer with stream/sink + service map
	//
	let mut peer = Peer::new( addr.clone(), stream_a.compat(), sink_a.sink_compat() ).expect( "spawn peer" );

	let evts = peer.observe( 10 );

	mb.start( peer ).expect( "Failed to start mailbox" );

	(addr, evts)
}



pub async fn connect_return_stream( socket: &str ) ->

	(TokSplitSink<Framed<TcpStream, MulServTokioCodec<MS>>>, TokSplitStream<Framed<TcpStream, MulServTokioCodec<MS>>>)

{
	// Connect to tcp server
	//
	let socket = socket.parse::<SocketAddr>().unwrap();
	let stream = await!( TcpStream::connect( &socket ).compat() ).expect( "connect address" );

	// frame the connection with codec for multiservice
	//
	let codec: MulServTokioCodec<MS> = MulServTokioCodec::new();

	codec.framed( stream ).split()
}




service_map!
(
	namespace:     remotes   ;
	peer_type:     MyPeer    ;
	multi_service: MS        ;
	services     : Add, Show ;
);

