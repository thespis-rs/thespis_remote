use
{
	thespis_remote_wasm_example_common :: *,
	thespis               :: { *               } ,
	thespis_impl          :: { *               } ,
	thespis_remote_impl   :: { *               } ,
	wasm_bindgen::prelude :: { *               } ,
	futures_codec        :: { *                } ,
	log::*,
	wasm_bindgen_futures :: { spawn_local      } ,

	ws_stream_wasm :: { *                      } ,
	pharos                :: { Observable, ObserveConfig } ,

	futures::
	{
		stream  :: { StreamExt                  } ,
	},
};

pub(crate) type Codec = MulServTokioCodec<MS>;

pub const URL   : &str = "ws://127.0.0.1:3012";


// Called when the wasm module is instantiated
//
#[ wasm_bindgen( start ) ]
//
pub fn main() -> Result<(), JsValue>
{
	wasm_logger::init( wasm_logger::Config::new( log::Level::Trace ));

	// console_log::init_with_level( log::Level::Trace ).expect( "initialize logger" );

	let program = async move
	{
		let (mut ws, wsio) = match WsStream::connect( URL, None ).await
		{
			Ok(conn) => conn,

			Err(e)   =>
			{
				// report error to the user
				//
				error!( "{}", e );
				panic!( "{}", e );
			}
		};

		let mut events = ws.observe( ObserveConfig::default() ).expect( "observe ws" );

		let ws_evts = async move
		{
			while let Some(e) = events.next().await
			{
				debug!( "ws event: {:?}", e );
			}
		};

		spawn_local( ws_evts );

		let framed      = Framed::new( wsio, Codec::new( 1024 /*max_length in bytes for a message*/ ) );
		let (out, msgs) = framed.split();

		// Create mailbox for server peer
		//
		let mb       : Inbox<Peer<MS>> = Inbox::new()             ;
		let peer_addr                  = Addr ::new( mb.sender() );

		// create peer with stream/sink + service map
		//
		let mut server = Peer::new( peer_addr.clone(), msgs, out ).expect_throw( "spawn peer" );

		let mut events = server.observe( ObserveConfig::default() ).expect( "observe ws" );

		let svr_evts = async move
		{
			while let Some(e) = events.next().await
			{
				debug!( "peer event: {:?}", e );
			}
		};

		spawn_local( svr_evts );


		let mut ping = remotes::Services::recipient::<Ping>( peer_addr.clone() );

		mb.start( server ).expect( "Failed to start mailbox" );


		// start new actor
		//
		// let mut addr = Addr::try_from( MyActor { count: 10 } ).expect( "create addres for MyActor" );

		// send message and get future for result
		//
		let res = ping.call( Ping(10) ).await.expect( "Send Ping" );

		let window   = web_sys::window  ().expect( "no global `window` exists"        );
		let document = window  .document().expect( "should have a document on window" );
		let body     = document.body    ().expect( "document should have a body"      );

		// Manufacture the element we're gonna append
		//
		let div = document.create_element( "div" ).expect( "Failed to create div" );

		div.set_inner_html( &format!( "The pong value is: {}", res ) );

		body.append_child( &div ).expect( "Coundn't append child" );
	};


	spawn_local( program );

	Ok(())
}
