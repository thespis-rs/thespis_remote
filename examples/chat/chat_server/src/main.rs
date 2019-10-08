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
		chat_format           :: { ChatMsg, SetNick, Join, ChatErr, ChatErrKind, ServerMsg, Welcome } ,
		chat_format           :: { client_map, server_map                                           } ,
		chrono                :: { Utc                                                              } ,
		futures_codec         :: { Framed                                                           } ,
		log                   :: { *                                                                } ,
		pin_utils             :: { pin_mut                                                          } ,
		regex                 :: { Regex                                                            } ,
		std                   :: { env, collections::HashMap, net::SocketAddr                       } ,
		warp                  :: { Filter                                                           } ,
		ws_stream_tungstenite :: { *                                                                } ,
		tokio_tungstenite     :: { WebSocketStream, accept_async                                    } ,
		thespis               :: { Mailbox, Message, Actor, Handler, Recipient, Return, Address     } ,
		thespis_impl          :: { Addr, Inbox, Receiver                                            } ,
		thespis_impl_remote   :: { MulServTokioCodec, MultiServiceImpl, PeerEvent                   } ,
		thespis_impl_remote   :: { peer::Peer, ConnID, ServiceID, Codecs                            } ,
		thespis_remote        :: { ServiceMap                                                       } ,
		pharos                :: { Observable, ObserveConfig                                        } ,
		tokio01               :: { net::{ TcpListener, TcpStream }                                  } ,
		futures01             :: { future::{ Future as Future01, ok }                               } ,

		futures::
		{
			select,
			compat  :: { Stream01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink                 } ,
			sink    :: { SinkExt                              } ,
			future  :: { FutureExt, TryFutureExt              } ,
		},
	};
}


use import::*;
use server::ConnectionClosed;


pub type TheSink    = SplitSink< Framed< WsStream<WebSocketStream<TcpStream>>, MulServTokioCodec<MS> >, MS> ;
pub type MS         = MultiServiceImpl<ServiceID, ConnID, Codecs>                                           ;



fn main()
{
	flexi_logger::Logger::with_str( "chat_server=trace, ws_stream=error, tokio=warn" ).start().unwrap();

	warp::spawn( accept_ws().unit_error().boxed().compat() );

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

	warp::spawn( mb_svr.start_fut( server ).unit_error().boxed().compat() );


	let socket = TcpListener::bind( &"127.0.0.1:3012".parse().unwrap() ).unwrap();
	let mut connections = socket.incoming().compat();

	while let Some( tcp ) = connections.next().await.transpose().expect( "tcp connection" )
	{
		let peer_addr = tcp.peer_addr().expect( "peer_addr" );
		let s = ok( tcp ).and_then( accept_async ).compat().await.expect( "ws handshake" );

		warp::spawn( handle_conn( s, peer_addr, sv_addr.clone() ).unit_error().boxed().compat() );
	}
}



// Runs once for each incoming connection, ends when the stream closes or sending causes an
// error.
//
async fn handle_conn( socket: WebSocketStream<TcpStream>, peer_addr: SocketAddr, server: Addr<Server> )
{
	info!( "Incoming connection from: {:?}", peer_addr );

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
	// We say that Sum from above will provide these services.
	// We just let the User actor handle all messages coming in from this client.
	//
	sm.register_handler::< Join    >( Receiver::new( usr_addr.recipient() ) );
	sm.register_handler::< SetNick >( Receiver::new( usr_addr.recipient() ) );
	sm.register_handler::< ChatMsg >( Receiver::new( usr_addr.recipient() ) );


	// register service map with peer
	// This tells this peer to expose all these services over this connection.
	//
	sm.register_with_peer( &mut peer );
	let peer_out  = client_map::Services::recipient::< ServerMsg >( cc_addr.clone() );

	let user = User::new( server.clone(), usr_addr.clone(), Box::new( peer_out ) );


	let mut peer_evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );
	let mut sa        = server.clone();
	let     sid       = usr_addr.id();

	let close_evt = async move
	{
		while let Some( PeerEvent::Closed ) = peer_evts.next().await
		{
			sa.send( ConnectionClosed(sid) ).await.expect( "notify connection closed" );
		}

	}.fuse();



	let mb_user2 = mb_user.start_fut( user ).fuse();
	let mb_peer2 = mb_peer.start_fut( peer ).fuse();

	pin_mut!( mb_user2  );
	pin_mut!( mb_peer2  );
	pin_mut!( close_evt );

	select!
	{
		_ = mb_user2  => {},
		_ = mb_peer2  => {},
		_ = close_evt => {},
	};
}

