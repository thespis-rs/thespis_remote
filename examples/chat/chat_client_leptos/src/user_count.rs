use std::{ rc::Rc, cell::RefCell };
use crate :: { import::*, color::*, document, Render, mount_to };
// use keke_dom::{ dom_manip::Manip, state::State };
use leptos::{ create_signal, view, component, Scope, ReadSignal, IntoView, WriteSignal, SignalUpdate, ScopeDisposer };

#[ derive( Actor ) ]
//
pub struct UserCount
{
	parent : HtmlElement  ,
	set_count: WriteSignal<usize>,
	count: ReadSignal<usize>,
}

impl UserCount
{
	pub fn new( parent: HtmlElement, cx: Scope ) -> Self
	{
		let (count, set_count) = create_signal(cx, 0);

		Self
		{
			count,
			set_count,
			parent,
		}
	}
}




pub struct Update { pub count: usize }

impl Message for Update { type Return = (); }


impl Handler< Update > for UserCount
{
	#[async_fn_local] fn handle_local( &mut self, msg: Update )
	{
		warn!("updating set_count");
		self.set_count.update( |n| *n = msg.count );
	}

	#[async_fn] fn handle( &mut self, _: Update )
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}


impl Handler< Render > for UserCount
{
	#[async_fn_local] fn handle_local( &mut self, Render{ cx }: Render )
	{
		let count = self.count;

		mount_to( cx, self.parent.clone(), move |cx|
		{
			view! { cx, <UserCountDom count=count /> }
		});

	}

	#[async_fn] fn handle( &mut self, _: Render )
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}


#[component]
fn UserCountDom( cx: Scope, count: ReadSignal<usize> ) -> impl IntoView
{
	warn!("running UserCountDom");
	view!
	{
		cx,

		<>Total users: {count}</>
	}
}
