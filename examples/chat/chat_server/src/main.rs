//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
#![ feature( async_closure ) ]

// mod error;
mod server;
mod user;

// use error::*;
use server::*;
use user::*;

mod import
{
	pub(crate) use
	{
		async_runtime         :: { rt                                                               } ,

		chat_format           :: { * } ,
		chat_format           :: { client_map, server_map                                           } ,
		chrono                :: { Utc                                                              } ,
		futures_codec         :: { Framed                                                           } ,
		log                   :: { *                                                                } ,
		regex                 :: { Regex                                                            } ,
		std                   :: { env, collections::HashMap, net::SocketAddr                       } ,
		warp                  :: { Filter as _                                                      } ,
		ws_stream_tungstenite :: { *                                                                } ,
		tokio_tungstenite     :: { WebSocketStream, accept_async                                    } ,
		thespis               :: { Mailbox, Message, Actor, Handler, Recipient, Return, Address     } ,
		thespis_impl          :: { Addr, Inbox, Receiver                                            } ,
		thespis_impl_remote   :: { MulServTokioCodec, MultiServiceImpl, PeerEvent                   } ,
		thespis_impl_remote   :: { peer::Peer, ConnID, ServiceID, Codecs                            } ,
		thespis_remote        :: { ServiceMap                                                       } ,
		pharos                :: { Observable, Filter                                               } ,
		tokio01               :: { net::{ TcpListener, TcpStream }                                  } ,
		futures01             :: { future::{ Future as Future01, ok }                               } ,

		futures::
		{
			compat  :: { Stream01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink                 } ,
			sink    :: { SinkExt                              } ,
		},
	};
}


use import::*;


pub type TheSink    = SplitSink< Framed< WsStream<WebSocketStream<TcpStream>>, MulServTokioCodec<MS> >, MS> ;
pub type MS         = MultiServiceImpl<ServiceID, ConnID, Codecs>                                           ;



fn main()
{
	flexi_logger::Logger::with_str( "warn, tungstenite=warn, mio=warn, ws_stream_tungstenite=warn, hyper=warn, tokio=warn" ).start().unwrap();

	rt::spawn( accept_ws() ).expect( "spawn accept_ws" );

	// let sv_addr = warp::any().map( move || sv_addr.clone() );

	// GET / -> index html
	//
	let index   = warp::path::end()        .and( warp::fs::file( "../chat_client/index.html" ) );
	let style   = warp::path( "style.css" ).and( warp::fs::file( "../chat_client/style.css"  ) );
	let statics = warp::path( "pkg"       ).and( warp::fs::dir ( "../chat_client/pkg"        ) );

	let routes  = index.or( style ).or( statics );


	let addr: SocketAddr = env::args().nth(1).unwrap_or( "127.0.0.1:3412".to_string() ).parse().expect( "valid addr" );
	println!( "server task listening at: {}", &addr );

	warp::serve( routes ).run( addr );
}



async fn accept_ws()
{
	// Create mailbox for peer
	//
	let mb_svr  : Inbox<Server> = Inbox::new()                 ;
	let sv_addr                 = Addr ::new( mb_svr.sender() );

	let server = Server::new();

	rt::spawn( mb_svr.start_fut( server ) ).expect( "spawn server mailbox" );


	let socket = TcpListener::bind( &"127.0.0.1:3012".parse().unwrap() ).unwrap();
	let mut connections = socket.incoming().compat();

	while let Some( tcp ) = connections.next().await.transpose().expect( "tcp connection" )
	{
		let peer_addr = tcp.peer_addr().expect( "peer_addr" );

		debug!( "incoming WS connection from {}", peer_addr );

		let s = ok( tcp ).and_then( accept_async ).compat().await.expect( "ws handshake" );

		rt::spawn( handle_conn( s, peer_addr, sv_addr.clone() ) ).expect( "spawn handle ws connection" );
	}
}



// Runs once for each incoming connection, ends when the stream closes or sending causes an
// error.
//
async fn handle_conn( socket: WebSocketStream<TcpStream>, peer_addr: SocketAddr, mut server: Addr<Server> )
{
	info!( "Handle WS connection from: {:?}", peer_addr );

	let codec: MulServTokioCodec<MS> = MulServTokioCodec::new( 32_000 );
	let ws_stream = WsStream::new( socket );

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
	let mb_user : Inbox<User> = Inbox::new()                  ;
	let usr_addr              = Addr ::new( mb_user.sender() );



	// Create a service map.
	// A service map is a helper object created by a beefy macro included with thespis_impl_remote. It is responsible
	// for deserializing and delivering the message to the correct handler.
	//
	let mut sm = server_map::Services::new();

	// Register our handlers.
	// We just let the User actor handle all messages coming in from this client.
	//
	sm.register_handler::< Join       >( Receiver::new( usr_addr.recipient() ) );
	sm.register_handler::< SetNick    >( Receiver::new( usr_addr.recipient() ) );
	sm.register_handler::< NewChatMsg >( Receiver::new( usr_addr.recipient() ) );


	// register service map with peer
	// This tells this peer to expose all these services over this connection.
	//
	sm.register_with_peer( &mut peer );

	let peer_out = client_map::Services::recipient::< ServerMsg >( cc_addr.clone() );
	let user     = User::new( server.clone(), usr_addr.clone(), Box::new( peer_out ) );

	let filter = Filter::Pointer( |e| *e == PeerEvent::Closed || *e == PeerEvent::ClosedByRemote );

	let close_evt = peer

		.observe( filter.into() )
		.expect( "pharos not closed" )
		.into_future()
	;


	rt::spawn( mb_user.start_fut( user ) ).expect( "spawn user mailbox" );
	rt::spawn( mb_peer.start_fut( peer ) ).expect( "spawn peer mailbox" );

	debug!( "Server handle wait close event" );

	close_evt.await;

	// Make sure we remove the user from the list
	//
	server.send( ConnectionClosed( usr_addr.id() ) ).await.expect( "close connection");

	debug!( "Server handle connection end" );
}

