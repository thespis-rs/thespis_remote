//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
#![ feature( async_closure ) ]



mod import
{
	pub(crate) use
	{
		thespis_remote_wasm_example_common :: *,

		async_runtime         :: { rt, RtConfig                                                     } ,
		futures_codec         :: { Framed                                                           } ,
		log                   :: { *                                                                } ,
		std                   :: { net::SocketAddr                       } ,
		ws_stream_tungstenite :: { *                                                                } ,
		tokio_tungstenite     :: { WebSocketStream, accept_async                                    } ,
		thespis               :: { Message, Actor, Handler, Return, Address, Mailbox     } ,
		thespis_impl          :: { Addr, Inbox, Receiver                                            } ,
		thespis_remote_impl   :: { MulServTokioCodec, MultiServiceImpl                   } ,
		thespis_remote_impl   :: { peer::Peer, ConnID, ServiceID, Codecs                            } ,
		thespis_remote        :: { ServiceMap                                                       } ,
		tokio01               :: { net::{ TcpListener, TcpStream }                                  } ,
		futures01             :: { future::{ Future as Future01, ok }                               } ,
		pharos                :: { Observable, Filter                                } ,

		futures::
		{
			compat  :: { Stream01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink                 } ,
		},
	};
}


use import::*;


pub type TheSink    = SplitSink< Framed< WsStream<WebSocketStream<TcpStream>>, MulServTokioCodec<MS> >, MS> ;
pub type MS         = MultiServiceImpl<ServiceID, ConnID, Codecs>                                           ;



fn main()
{

	flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	// let mut runtime = Runtime::new().unwrap();
	let program = async move
	{
		let socket = TcpListener::bind( &"127.0.0.1:3012".parse().unwrap() ).unwrap();
		let mut connections = socket.incoming().compat();

		while let Some( tcp ) = connections.next().await.transpose().expect( "tcp connection" )
		{
			let peer_addr = tcp.peer_addr().expect( "peer_addr" );

			debug!( "incoming WS connection from {}", peer_addr );

			let s = ok( tcp ).and_then( accept_async ).compat().await.expect( "ws handshake" );

			rt::spawn( handle_conn( s, peer_addr ) ).expect( "spawn connection" );
		}
	};

	rt::block_on( program );
}


// Runs once for each incoming connection, ends when the stream closes or sending causes an
// error.
//
async fn handle_conn( socket: WebSocketStream<TcpStream>, peer_addr: SocketAddr )
{
	info!( "Handle WS connection from: {:?}", peer_addr );

	let codec: MulServTokioCodec<MS> = MulServTokioCodec::new( 32_000 );
	let mut ws_stream = WsStream::new( socket );


	let closed = Filter::Pointer( |e|
	{
		match e
		{
			WsEvent::Closed => true,
			_               => false,
		}
	});

	let mut events = ws_stream.observe( closed.into() ).expect( "observe ws_stream" );
	let (out, msgs) = Framed::new( ws_stream, codec ).split();

	// Create mailbox for peer
	//
	let mb_peer: Inbox<Peer<MS>> = Inbox::new()                  ;
	let cc_addr                  = Addr ::new( mb_peer.sender() );

	// create peer with stream/sink
	//
	let mut peer = Peer::new( cc_addr.clone(), msgs, out ).expect( "create peer" );


	// Create mailbox for user
	//
	let mb_actor: Inbox<MyActor> = Inbox::new()                    ;
	let actor                    = Addr ::new( mb_actor.sender() ) ;

	let my_act = MyActor { count: 5 };

	// Create a service map.
	// A service map is a helper object created by a beefy macro included with thespis_remote_impl. It is responsible
	// for deserializing and delivering the message to the correct handler.
	//
	let mut sm = remotes::Services::new();

	// Register our handlers.
	// We just let the User actor handle all messages coming in from this client.
	//
	sm.register_handler::< Ping >( Receiver::new( actor.recipient() ) );

	// register service map with peer
	//
	sm.register_with_peer( &mut peer );

	mb_peer .start( peer   ).expect( "Failed to start mailbox of Peer"    );
	mb_actor.start( my_act ).expect( "Failed to start mailbox of MyActor" );

	events.next().await;

	info!( "connection closed" );
}





/// Actor
//
#[ derive( Actor ) ]
//
pub struct MyActor { pub count: usize }



/// Handler for `Ping` message
//
impl Handler<Ping> for MyActor
{
	fn handle( &mut self, msg: Ping ) -> Return<<Ping as Message>::Return>
	{
		Box::pin( async move
		{
			self.count += msg.0;
			self.count
		})
	}
}
