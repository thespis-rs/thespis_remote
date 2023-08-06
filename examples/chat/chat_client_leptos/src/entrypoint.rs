#![ allow( unused_imports, clippy::suspicious_else_formatting, clippy::needless_return ) ]

pub(crate) mod chat_form    ;
pub(crate) mod chat_window  ;
pub(crate) mod connect_form ;
pub(crate) mod color        ;
pub(crate) mod user         ;
pub(crate) mod user_list    ;
pub(crate) mod app          ;

use
{
	connect_form :: { ConnectForm, ConnectFormDom, ConnSubmitEvt, ConnResetEvt                       } ,
	chat_form    :: { ChatForm, ChatFormDom, ChatSubmitEvt, ChatResetEvt                             } ,
	app          :: { App, Connected, Disconnect                                                     } ,
	chat_window  :: { ChatWindow, ChatWindowDom, NewUser, UserLeft, AnnounceNick, ChatMsg, PrintHelp } ,
	user         :: { ChangeNick, UserInfo, User                                                     } ,
	user_list    :: { UserListDom                                                                    } ,
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
		leptos               :: { create_effect, create_memo, create_signal, view, component, For, Memo, Scope, ScopeDisposer, ReadSignal, IntoView, WriteSignal, SignalUpdate, SignalGet, SignalGetUntracked } ,
		leptos_use           :: {  } ,
		leptos_dom           :: { Mountable                                                             } ,
		tracing              :: { *                                                                     } ,
		web_sys              :: { Event                                                                 } ,
		wasm_bindgen         :: { prelude::*, JsCast                                                    } ,
		gloo_events          :: { EventListener, EventListenerOptions                                   } ,
		std                  :: { rc::Rc, convert::{TryInto, TryFrom}, cell::{OnceCell, RefCell}, io, sync::Arc     } ,
		std                  :: { task::*, pin::Pin, collections::HashMap, panic                        } ,
		regex                :: { Regex                                                                 } ,
		js_sys               :: { Date, Math                                                            } ,
		pin_utils            :: { pin_mut                                                               } ,
		wasm_bindgen_futures :: { spawn_local                                                           } ,

		chat_format         :: { * } ,
		thespis             :: { Message, Actor, Handler, Return, ReturnNoSend, Address } ,
		thespis_derive      :: { async_fn_local, async_fn_nosend, async_fn } ,
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
		color::*,
		user_list::*,
	}
};


thread_local!
{
	pub static CX: OnceCell<Scope> = OnceCell::new();
}





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

	leptos::mount_to_body( |cx|
	{
		CX.with(|cell| cell.set(cx) ).expect_throw( "set CX" );
		view!{ cx, <></> }
	});

	let (app_addr, app_mb) = Addr::builder( "App" ).build();
	let app = App::new(app_addr);

	Bindgen.spawn_local( async{ app_mb.start_local( app ).await; } )
		.expect_throw( "spawn app" )
	;

	info!( "main function ends" );

	Ok(())
}


pub(crate) fn document() -> Document
{
	let window = web_sys::window().expect_throw( "no global `window` exists");

	window.document().expect_throw( "should have a document on window" )
}

pub(crate) fn get_id( id: &str ) -> HtmlElement
{
	document().get_element_by_id( id ).expect_throw( &format!( "find {}", id ) ).unchecked_into()
}


/// Which operation to use to mount a leptos view.
//
pub enum Operation
{
	Append,
	Prepend,
	InsertBefore,
	InsertAfter,
}

/// Runs the provided closure and mounts the result to the provided element,
/// creating a child context.
//
pub fn mount_to<F, N>( cx: Scope, ref_node: HtmlElement, op: Operation, f: F ) -> HtmlElement
where
    F: FnOnce(Scope) -> N + 'static,
    N: IntoView,
{
	let node = f(cx).into_view(cx);
	let elem = node.get_mountable_node();

	match op
	{
		Operation::Append       => { ref_node.append_child(&elem).unwrap(); }
		Operation::Prepend      => { ref_node.prepend_with_node(&elem.clone().unchecked_into()).unwrap(); }
		Operation::InsertBefore => { ref_node.parent_node().unwrap().insert_before(&elem, Some(&ref_node)).unwrap(); }
		Operation::InsertAfter  => { ref_node.after_with_node_1(&elem).unwrap(); }
	}

	// leptos first inserts a comment node with the name of the component.
	//
	let elem = node.get_opening_node().next_sibling().expect_throw("find node we inserted");

	elem.unchecked_into()
}



/// Runs the provided closure and mounts the result to the provided element.
//
pub fn mount_to_global<F, N>(ref_node: HtmlElement, op: Operation, f: F) -> HtmlElement
where
    F: FnOnce(Scope) -> N + 'static,
    N: IntoView,
{
	let cx = CX.with( |cx| *cx.get().expect_throw( "cx to be created" ) );

	mount_to( cx, ref_node, op, f )
}



// Return a random name
//
pub(crate) fn random_name() -> &'static str
{
	// I wanted to use the crate scottish_names to generate a random username, but
	// it uses the rand crate which doesn't support wasm for now, so we're just using
	// a small sample.
	//
	let list =
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
