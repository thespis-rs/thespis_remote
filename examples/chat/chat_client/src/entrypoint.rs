#![ allow( unused_imports, clippy::suspicious_else_formatting, clippy::needless_return ) ]

pub(crate) mod chat_form    ;
pub(crate) mod chat_window  ;
pub(crate) mod connect_form ;
pub(crate) mod e_handler    ;
pub(crate) mod color        ;
pub(crate) mod user         ;
pub(crate) mod user_count   ;
pub(crate) mod user_list    ;
pub(crate) mod app          ;

use
{
	connect_form :: { ConnectForm, ConnSubmitEvt, ConnResetEvt                        } ,
	chat_form    :: { ChatForm, ChatSubmitEvt, ChatResetEvt                           } ,
	app          :: { App, Connected, Disconnect                                      } ,
	chat_window  :: { ChatWindow, NewUser, UserLeft, AnnounceNick, ChatMsg, PrintHelp } ,
	user         :: { ChangeNick, UserInfo, User                                      } ,
	user_count   :: { UserCount                                                       } ,
};

mod import
{
	pub(crate) use
	{
		async_executors      :: { Bindgen                                                               } ,
		web_sys              :: { *, console::log_1 as dbg                                              } ,
		ws_stream_wasm       :: { *                                                                     } ,
		futures              :: { prelude::*, stream::SplitStream, select, future::ready, FutureExt     } ,
		futures              :: { channel::{ mpsc::{ unbounded, UnboundedReceiver, UnboundedSender } }  } ,
		futures              :: { task::LocalSpawnExt                                                   } ,
		tracing              :: { *                                                                     } ,
		web_sys              :: { Event, HtmlElement                                                    } ,
		wasm_bindgen         :: { prelude::*, JsCast                                                    } ,
		gloo_events          :: { EventListener, EventListenerOptions                                   } ,
		std                  :: { rc::Rc, convert::{TryInto, TryFrom}, cell::RefCell, io, sync::Arc     } ,
		std                  :: { task::*, pin::Pin, collections::HashMap, panic                        } ,
		regex                :: { Regex                                                                 } ,
		js_sys               :: { Date, Math                                                            } ,
		pin_utils            :: { pin_mut                                                               } ,
		wasm_bindgen_futures :: { spawn_local                                                           } ,

		chat_format         :: { * } ,
		thespis             :: { Message, Actor, Handler, Return, ReturnNoSend, Address, async_fn, async_fn_local } ,
		thespis_impl        :: { Addr, WeakAddr                                                                   } ,
		thespis_remote      :: { CborWF, PeerEvent, Peer, ServiceMap, WireFormat                                  } ,
		pharos              :: { Observable, ObserveConfig, Events                                                } ,
	};
}

use
{
	// tracing_subscriber::fmt,
	// tracing_subscriber_wasm::MakeConsoleWriter,

	tracing_web::{MakeConsoleWriter, performance_layer},
	tracing_subscriber::fmt::format::Pretty,
	tracing_subscriber::fmt::time::UtcTime,
	tracing_subscriber::prelude::*,


	crate::
	{
		import::*,
		e_handler::*,
		color::*,
		user_list::*,
	}
};



// Called when the wasm module is instantiated
//
#[ wasm_bindgen( start ) ]
//
pub async fn main() -> Result<(), JsValue>
{
	panic::set_hook( Box::new(console_error_panic_hook::hook) );

	// fmt()
	// 	.with_writer(
	// 		// To avoide trace events in the browser from showing their
	// 		// JS backtrace, which is very annoying, in my opinion
	// 		MakeConsoleWriter::default().map_trace_level_to(tracing::Level::DEBUG),
	// 	)
	// 	// For some reason, if we don't do this in the browser, we get
	// 	// a runtime error.
	// 	.with_ansi(false)
	// 	.without_time()
	// 	.init()
	// ;

	// Let's only log output when in debug mode
	//
	// #[ cfg( debug_assertions ) ]
	//
	// wasm_logger::init( wasm_logger::Config::new(log::Level::Trace).message_on_new_line() );
	//

	let fmt_layer = tracing_subscriber::fmt::layer()
		.with_ansi(false) // Only partially supported across browsers
		// .with_timer(UtcTime::rfc_3339()) // std::time is not available in browsers
		.without_time()
		.with_writer(MakeConsoleWriter) // write events to the console
		.with_filter(tracing::level_filters::LevelFilter::INFO)
		;
	// let perf_layer = performance_layer()
	// 	.with_details_from_fields(Pretty::default());

	tracing_subscriber::registry()
		.with(fmt_layer)
		// .with(perf_layer)
		.init(); // Install these as subscribers to tracing events

	trace!( "This is a trace in main" );

	let (app_addr, app_mb) = Addr::builder( "App" ).build();
	let app = App::new( app_addr);

	app_mb.start_local( app ).await;

	info!( "main function ends" );

	Ok(())
}







pub(crate) fn document() -> Document
{
	let window = web_sys::window().expect_throw( "no global `window` exists");

	window.document().expect_throw( "should have a document on window" )
}

pub(crate) fn get_id( id: &str ) -> Element
{
	document().get_element_by_id( id ).expect_throw( &format!( "find {}", id ) )
}




// Return a random name
//
pub(crate) fn random_name() -> &'static str
{
	// I wanted to use the crate scottish_names to generate a random username, but
	// it uses the rand crate which doesn't support wasm for now, so we're just using
	// a small sample.
	//
	let list = vec!
	[
			"Aleeza"
		, "Aoun"
		, "Arya"
		, "Azaan"
		, "Ebony"
		, "Emke"
		, "Elena"
		, "Hafsa"
		, "Hailie"
		, "Inaaya"
		, "Iqra"
		, "Kobi"
		, "Noor"
		, "Nora"
		, "Nuala"
		, "Orin"
		, "Pippa"
		, "Rhuaridh"
		, "Salah"
		, "Susheela"
		, "Teya"
	];

	// pick one
	//
	list[ Math::floor( Math::random() * list.len() as f64 ) as usize ]
}
