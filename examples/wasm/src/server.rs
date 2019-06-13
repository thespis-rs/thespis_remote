#![ feature( async_await ) ]

mod common;

use
{
	common::*,
	tokio::net::TcpListener,
	tokio_tungstenite::accept_async,
};

pub const LISTEN: &str = "127.0.0.1:3012"     ;



fn main()
{
	let program = async
	{
		println!( "Listening on {}", LISTEN );

		let     addr   = LISTEN.parse().unwrap();
		let mut socket = TcpListener::bind( &addr ).unwrap().incoming().compat();

		let conn = socket.next().await.expect( "tcp stream" ).expect( "tcp stream" );

		let ws_stream = accept_async( conn ).compat().await.expect( "websocket connection" );

		// frame the connection with codec for multiservice
		//
		let codec: MulServTokioCodec<MS> = MulServTokioCodec::new(1024);

		// let (tx, rx) = codec.framed( ws_stream ).split();
	};

	rt::spawn( program ).expect( "Spawn program" );
	rt::run();

}



