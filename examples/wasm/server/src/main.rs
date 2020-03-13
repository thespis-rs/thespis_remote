//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
mod import
{
	pub(crate) use
	{
		thespis_remote_wasm_example_common :: *,

		async_executors       :: { AsyncStd                                 } ,
		async_std             :: { net::{ TcpListener, TcpStream }          } ,
		async_tungstenite     :: { accept_async, WebSocketStream            } ,
		log                   :: { *                                        } ,
		std                   :: { net::SocketAddr, sync::Arc               } ,
		ws_stream_tungstenite :: { *                                        } ,
		thespis               :: { Message, Actor, Handler, Return, Address } ,
		thespis_impl          :: { Addr, Inbox                              } ,
		thespis_remote        :: { peer::Peer                               } ,
		pharos                :: { Observable, Filter                       } ,

		futures::
		{
			stream  :: { StreamExt                                           } ,
		},
	};
}


use import::*;


// pub type TheSink    = SplitSink< Framed< WsStream<WebSocketStream<TcpStream>>, MulServTokioCodec<MS> >, MS> ;
// pub type MS         = MultiServiceImpl<ServiceID, ConnID, Codecs>                                           ;


#[ async_std::main ]
//
async fn main()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let     socket      = TcpListener::bind( "127.0.0.1:3012" ).await.expect( "bind to port" );
	let mut connections = socket.incoming();

	while let Some( tcp ) = connections.next().await.transpose().expect( "tcp connection" )
	{
		let peer_addr = tcp.local_addr().expect( "peer_addr" );

		debug!( "incoming WS connection from {}", peer_addr );

		let s = accept_async( tcp ).await.expect("Error during the websocket handshake occurred");

		async_std::task::spawn( handle_conn( s, peer_addr ) );
	}
}


// Runs once for each incoming connection, ends when the stream closes or sending causes an
// error.
//
async fn handle_conn( socket: WebSocketStream<TcpStream>, peer_addr: SocketAddr )
{
	info!( "Handle WS connection from: {:?}", peer_addr );

	let exec = AsyncStd::new();
	let mut ws_stream = WsStream::new( socket );


	let closed = Filter::Pointer( |e| matches!( e, WsEvent::Closed ) );

	let mut events = ws_stream.observe( closed.into() ).expect( "observe ws_stream" );

	// Create mailbox for peer
	//
	let mb_peer: Inbox<Peer> = Inbox::new( Some( format!( "{:?}", peer_addr ).into() ) );
	let cc_addr              = Addr ::new( mb_peer.sender() );

	// create peer with stream/sink
	//
	let mut peer = Peer::from_async_read( cc_addr.clone(), ws_stream, 1024, exec, None ).expect( "create peer" );


	// Create mailbox for user
	//
	let mb_actor: Inbox<MyActor> = Inbox::new( Some( "my actor".into() ) );
	let actor                    = Addr ::new( mb_actor.sender() ) ;

	let my_act = MyActor { count: 5 };

	// Create a service map.
	// A service map is a helper object created by a beefy macro included with thespis_remote_impl. It is responsible
	// for deserializing and delivering the message to the correct handler.
	//
	let sm = remotes::Services::new();

	// Register our handlers.
	// We just let the User actor handle all messages coming in from this client.
	//
	sm.register_handler::< Ping >( actor.clone_box() );

	// register service map with peer
	//
	peer.register_services( Arc::new( sm ) );

	mb_peer .start( peer  , &exec ).expect( "Failed to start mailbox of Peer"    );
	mb_actor.start( my_act, &exec ).expect( "Failed to start mailbox of MyActor" );

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
