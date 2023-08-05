use std::{ rc::Rc, cell::RefCell };
use crate :: { import::*, color::*, document, user_list::Render };
// use keke_dom::{ dom_manip::Manip, state::State };
use leptos::{ create_signal, view, component, Scope, ReadSignal, IntoView, WriteSignal, SignalUpdate };

#[ derive( Actor ) ]
//
pub struct UserCount
{
	_parent : HtmlElement  ,
	set_count: WriteSignal<usize>,
}

impl UserCount
{
	pub fn new( parent: HtmlElement ) -> Self
	{
		let set_count: Rc<RefCell<Option<WriteSignal<usize>>>> = Rc::new(None.into());
		let set_count2 = set_count.clone();

		leptos::mount_to( parent.clone(), move |cx|
		{
			let (count, set_count_inner) = create_signal(cx, 0);

			set_count.replace(Some(set_count_inner));
			warn!("inserted set_count");


			view! { cx, <UserCountDom count=count /> }
		});

		let set_count = set_count2.borrow_mut().take().expect_throw("unwrap set_count");

		Self
		{
			set_count,
			_parent: parent,
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
	#[async_fn_local] fn handle_local( &mut self, _: Render )
	{
		// self.count.get_inner_text( self.parent.clone() );


		// rsx
		// (r#"
		// 	<div id="user_count">
		// 		{ self.count }
		// 	</div>
		// "#)

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
