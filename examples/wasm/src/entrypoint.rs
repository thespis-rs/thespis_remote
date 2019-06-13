#![ feature( async_await ) ]

#![ allow( dead_code, unused_imports, unused_variables ) ]

mod common;

use common::*;

pub const URL   : &str = "ws://localhost:3012";


// Called when the wasm module is instantiated
//
#[ wasm_bindgen( start ) ]
//
pub fn main() -> Result<(), JsValue>
{
	console_log::init_with_level( log::Level::Trace ).expect( "initialize logger" );

	let program = async move
	{
		let ws = connect().await;

		// frame the connection with codec for multiservice
		//
		let codec: MulServTokioCodec<MS> = MulServTokioCodec::new(1024);

		let (tx, rx) = codec.framed( ws ).split();

		// Create mailbox for server peer
		//
		let mb  : Inbox<MyPeer> = Inbox::new()             ;
		let addr                = Addr ::new( mb.sender() );

		// create peer with stream/sink + service map
		//
		let server = Peer::new( addr.clone(), rx.compat(), tx.sink_compat() ).expect( "spawn peer" );

		// let evts = server.observe( 10 );

		let mut ping   = remotes::Services::recipient::<Ping>( addr );


		mb.start( server ).expect( "Failed to start mailbox" );


		// start new actor
		//
		// let mut addr = Addr::try_from( MyActor { count: 10 } ).expect( "create addres for MyActor" );

		// send message and get future for result
		//
		let res = ping.call( Ping(5) ).await.expect( "Send Ping" );

		let window   = web_sys::window  ().expect( "no global `window` exists"        );
		let document = window  .document().expect( "should have a document on window" );
		let body     = document.body    ().expect( "document should have a body"      );

		// Manufacture the element we're gonna append
		//
		let div = document.create_element( "div" ).expect( "Failed to create div" );

		div.set_inner_html( &format!( "The pong value is: {}", res ) );

		body.append_child( &div ).expect( "Coundn't append child" );
	};


	rt::spawn( program ).expect( "spawn program" );
	rt::run();

	Ok(())
}




async fn connect() -> WsStream
{
	WsStream::connect( URL )

		.await
		.map_err     ( |e| { debug!( "{:?}", &e ); e }        )
		.expect_throw( "Couldn't create websocket connection" )
}
