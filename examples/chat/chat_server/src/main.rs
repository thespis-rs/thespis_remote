//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
#![allow(clippy::suspicious_else_formatting)]
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
		async_executors       :: { TokioTpBuilder, TokioTp                                } ,
		async_tungstenite     :: { accept_async, tokio::{ TokioAdapter }, WebSocketStream } ,
		chat_format           :: { *                                                      } ,
		chrono                :: { Utc                                                    } ,
		log                   :: { *                                                      } ,
		regex                 :: { Regex                                                  } ,
		std                   :: { env, collections::HashMap, net::SocketAddr, sync::Arc  } ,
		warp                  :: { Filter as _                                            } ,
		ws_stream_tungstenite :: { *                                                      } ,
		thespis               :: { Message, Actor, Handler, Return, Address, Identify, async_fn     } ,
		thespis_impl          :: { Addr                                                   } ,
		thespis_remote        :: { CborWF, PeerEvent, WireFormat                          } ,
		pharos                :: { Observable, Filter                                     } ,
		tokio                 :: { net::{ TcpListener, TcpStream }                        } ,


		futures::
		{
			future  :: { FutureExt } ,
			stream  :: { StreamExt } ,
			task    :: { SpawnExt  } ,
			sink    :: { SinkExt   } ,
		},
	};
}


use import::*;

fn main()
{
	let exec = TokioTpBuilder::new().build().expect( "create tokio threadpool" );

	flexi_logger::Logger::try_with_str( "warn, tungstenite=warn, mio=warn, ws_stream_tungstenite=warn, hyper=warn, tokio=warn" ).expect("flexi_logger").start().expect("flexi_logger");

	exec.spawn( accept_ws(exec.clone()) ).expect( "spawn accept_ws" );

	exec.block_on( async {
		// GET / -> index html
		//
		let index   = warp::path::end()        .and( warp::fs::file( "../chat_client_leptos/index.html" ) );
		let style   = warp::path( "style.css" ).and( warp::fs::file( "../chat_client_leptos/style.css"  ) );
		let statics = warp::path( "pkg"       ).and( warp::fs::dir ( "../chat_client_leptos/pkg"        ) );

		let routes  = index.or( style ).or( statics );

		let addr: SocketAddr = env::args().nth(1).unwrap_or( "127.0.0.1:3412".to_string() ).parse().expect( "valid addr" );
		println!( "server task listening at: {}", &addr );

		warp::serve( routes ).run( addr ).await;
	})

}


async fn accept_ws( exec: TokioTp )
{
	// Create mailbox for peer
	//
	let sv_addr = Addr::builder("server").spawn( Server::new(), &exec).expect("spawn server");

	let socket = TcpListener::bind( "127.0.0.1:3012" ).await.expect("listen for ws conn");

	loop
	{
		let tcp = socket.accept().await;

		// If the TCP stream fails, we stop processing this connection
		//
		let (tcp_stream, peer_addr) = match tcp
		{
			Ok( tuple ) => tuple,

			Err(e) =>
			{
				debug!( "Failed TCP incoming connection: {}", e );
				return;
			}
		};

		let s = accept_async( TokioAdapter::new(tcp_stream) ).await;

		// If the Ws handshake fails, we stop processing this connection
		//
		let socket = match s
		{
			Ok(ws) => ws,

			Err(e) =>
			{
				debug!( "Failed WebSocket HandShake: {}", e );
				return;
			}
		};


		info!( "Incoming connection from: {}", peer_addr );

		exec.spawn( handle_conn( socket, peer_addr, sv_addr.clone(), exec.clone() ) ).expect( "spawn handle ws connection" );
	}
}



// Runs once for each incoming connection, ends when the stream closes or sending causes an
// error.
//
async fn handle_conn
(
	socket    : WebSocketStream<TokioAdapter<TcpStream>>,
	peer_addr : SocketAddr,
	mut server: Addr<Server>,
	exec      : TokioTp,
)
{
	info!( "Handle WS connection from: {:?}", peer_addr );

	let ws_stream = WsStream::new( socket );

	let (mut peer, peer_mb, peer_addr) = CborWF::create_peer
	(
		"client",
		ws_stream,
		10_000_000, // 10 MB
		10_000_000, // 10 MB
		exec.clone(),
		None,
		None,
	).expect( "create server peer" );

	// Create mailbox for user
	//
	let (usr_addr, usr_mb) = Addr::builder("user").build();

	// Create a service map.
	// A service map is a helper object created by a beefy macro included with thespis_remote_impl. It is responsible
	// for deserializing and delivering the message to the correct handler.
	//
	let mut sm = server_map::Services::new();

	// Register our handlers.
	// We just let the User actor handle all messages coming in from this client.
	//
	sm.register_handler::< Join       >( usr_addr.clone_box() );
	sm.register_handler::< SetNick    >( usr_addr.clone_box() );
	sm.register_handler::< NewChatMsg >( usr_addr.clone_box() );


	// register service map with peer
	// This tells this peer to expose all these services over this connection.
	//
	peer.register_services( Arc::new(sm));

	let peer_out = client_map::RemoteAddr::new(peer_addr);
	let user     = User::new( server.clone(), usr_addr.clone(), peer_out );

	let filter = Filter::Pointer( |e| *e == PeerEvent::Closed || *e == PeerEvent::ClosedByRemote );

	let close_evt = peer

		.observe( filter.into() )
		.await
		.expect( "pharos not closed" )
		.into_future()
	;


	exec.spawn( usr_mb .start( user ).map(|_|()) ).expect( "spawn user mailbox" );
	exec.spawn( peer_mb.start( peer ).map(|_|()) ).expect( "spawn peer mailbox" );

	debug!( "Server handle wait close event" );

	close_evt.await;

	// Make sure we remove the user from the list
	//
	let user_id = usr_addr.id();
	server.send( ConnectionClosed( user_id ) ).await.expect( "close connection");

	debug!( "Server handle connection end" );
}

