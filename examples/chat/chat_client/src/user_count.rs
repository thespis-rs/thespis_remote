use crate :: { import::*, color::*, document, user_list::Render };
use keke_dom::{ dom_manip::Manip, state::State };


#[ derive( Actor ) ]
//
pub struct UserCount
{
	count  : State ,
	// indom  : bool         ,
	parent : HtmlElement  ,
}

impl UserCount
{
	pub fn new( parent: HtmlElement ) -> Self
	{
		Self
		{
			count: State::default(),
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
		self.count.set( format!("Total users: {}", msg.count) );
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
		self.count.get_inner_text( self.parent.clone() );

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
