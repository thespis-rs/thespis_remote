//! This is an echo server that returns all incoming bytes, without framing. It is used for the tests in
//! ws_stream_wasm.
//
#![ feature( async_closure ) ]

// mod error;
// use error::*;

mod import
{
	pub( crate ) use
	{
		chat_format        :: { ClientMsg, ServerMsg                                       } ,
		chrono             :: { Utc                                                        } ,
		futures            :: { select, sink::SinkExt, future::{ FutureExt, TryFutureExt } } ,
		futures_cbor_codec :: { Codec                                                      } ,
		futures_codec      :: { Framed, FramedRead, FramedWrite                            } ,
		locks::rwlock      :: { RwLock, read::FutureReadable, write::FutureWriteable       } ,
		log                :: { *                                                          } ,
		pin_utils          :: { pin_mut                                                    } ,
		regex              :: { Regex                                                      } ,
		std                :: { env, collections::HashMap, net::SocketAddr, error::Error as ErrorTrait } ,
		std::sync          :: { Arc, atomic::{ AtomicUsize, Ordering }                     } ,
		warp               :: { Filter                                                     } ,
		ws_stream          :: { *                                                          } ,

		futures ::
		{
			StreamExt                                     ,
			channel::mpsc::{ unbounded, UnboundedSender } ,
		},
	};
}


mod connection;
mod user      ;
mod clients   ;

pub use connection :: * ;
pub use user       :: * ;
pub use clients    :: * ;

use import::*;

static WELCOME : &str = "Welcome to the ws_stream Chat Server!" ;


fn main()
{
	flexi_logger::Logger::with_str( "server=trace, ws_stream=error, tokio=warn" ).start().unwrap();


	// This should be unfallible, so we ignore the result
	//
	let clients = Clients::new();


	// GET /chat -> websocket upgrade
	//
	let chat = warp::path( "chat" )

		// The `ws2()` filter will prepare Websocket handshake...
		//
		.and( warp::ws2() )

		.and( warp::addr::remote() )
		.and( clients              )

		.map( |ws: warp::ws::Ws2, peer_addr: Option<SocketAddr>, clients|
		{
			// This will call our function if the handshake succeeds.
			//
			ws.on_upgrade( move |socket|

				handle_conn( WarpWebSocket::new(socket), peer_addr.expect( "need peer_addr" ), clients ).boxed().compat()
			)
		})
	;

	// GET / -> index html
	//
	let index   = warp::path::end()        .and( warp::fs::file( "../chat_client/index.html" ) );
	let style   = warp::path( "style.css" ).and( warp::fs::file( "../chat_client/style.css"  ) );
	let statics = warp::path( "pkg" )      .and( warp::fs::dir ( "../chat_client/pkg"        ) );

	let routes  = index.or( style ).or( chat ).or( statics );


	let addr: SocketAddr = env::args().nth(1).unwrap_or( "127.0.0.1:3412".to_string() ).parse().expect( "valid addr" );
	println!( "server task listening at: {}", &addr );

	warp::serve( routes ).run( addr );
}



// Runs once for each incoming connection, ends when the stream closes or sending causes an
// error.
//
async fn handle_conn( socket: WarpWebSocket, peer_addr: SocketAddr, clients: Clients ) -> Result<(), ()>
{
	let conn = Connection::new( socket, peer_addr );

	let user = match User::from_connection( conn, &clients ).await
	{
		Ok(user) => { user }
		Err(e) => { return Err(())}
	};


	// Let all clients know there is a new kid on the block
	//
	clients.broadcast( &ServerMsg::UserJoined { time: Utc::now().timestamp(), nick: user.nick.read().clone(), sid: user.sid } );

	let sid = user.sid;
	assert!( clients.insert( user ).is_none() );

	// Welcome message
	//
	trace!( "sending welcome message" );

	let welcome = ServerMsg::Welcome
	{
		time : Utc::now().timestamp() ,
		txt  : WELCOME.to_string()    ,
		users: clients.users()        ,
	};

	clients.send( sid, ServerMsg::JoinSuccess ).await;
	clients.send( sid, welcome ).await;


	// Listen to the channel for this connection and sends out each message that
	// arrives on the channel.
	//
	// let outgoing = async move
	// {
	// 	let res = rx.map( |res| Ok( res ) ).forward( out ).await;

	// 	match res
	// 	{
	// 		Ok (_) => {}
	// 		Err(e) =>
	// 		{
	// 			error!( "{}", e );
	// 		}
	// 	}
	// };


	clients.start( sid ).await;


	pin_mut!( incoming );
	pin_mut!( outgoing );

	select!
	{
		_ = incoming.fuse() => {}
		_ = outgoing.fuse() => {}
	};


	// remove the client and let other clients know this client disconnected
	//
	let user = { CONNS.get().unwrap().future_write().await.remove( &peer_addr ) };

	if user.is_some()
	{
		// let other clients know this client disconnected
		//
		broadcast( &ServerMsg::UserLeft { time: Utc::now().timestamp(), nick: nick2.future_read().await.clone(), sid } );


		debug!( "Client disconnected: {}", peer_addr );
	}


	info!( "End of connection from: {:?}", peer_addr );

	Ok(())

}










// fn random_id() -> String
// {
// 	thread_rng()
// 		.sample_iter( &Alphanumeric )
// 		.take(8)
// 		.collect()
// }

