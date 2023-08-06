use crate::{ import::*, color::*, document, mount_to, CX, Operation };


#[component]
pub fn UserDom( cx: Scope, style: String, nick: ReadSignal<String>, sid: usize  ) -> impl IntoView
{
	let id = format!( "user_{}", sid );

	view! { cx,

		<li id={id} style={style}>{nick}</li>
	}
}



#[ derive( Actor ) ]
//
pub struct User
{
	color         : Color                ,
	sid           : usize                ,
	li            : HtmlElement          ,
	set_nick      : WriteSignal<String>  ,
	nick          : ReadSignal<String>   ,
	scope_disposer: Option<ScopeDisposer>,
}


impl User
{
	pub fn new( sid: usize, nick: String, parent: HtmlElement, is_self: bool ) -> Self
	{
		let color = Color::random().light();
		let mut style = format!( "color: {};", color.to_css() );

		if is_self { style.push_str( " font-weight: bold;"); }

		let global_cx = CX.with( |cx| *cx.get().expect_throw( "cx to be created" ) );
		let (child_cx, scope_disposer) = global_cx.run_child_scope( |cx| cx );

		let (nick, set_nick) = create_signal( child_cx, nick );

		let li = mount_to( child_cx, parent.clone(), Operation::Append, move |cx|
		{
			view! { cx, <UserDom style=style nick=nick sid=sid /> }
		});

		Self
		{
			color,
			li,
			nick,
			set_nick,
			scope_disposer: Some(scope_disposer),
			sid   ,
		}
	}
}



impl Drop for User
{
	fn drop( &mut self )
	{
		self.li.remove();

		if let Some(sd) = self.scope_disposer.take() { sd.dispose(); }
	}
}




pub struct UserInfo {}


impl Message for UserInfo { type Return = (usize, String, Color); }


impl Handler< UserInfo > for User
{
	#[async_fn_nosend] fn handle_local( &mut self, _: UserInfo ) -> (usize, String, Color)
	{
		(self.sid, self.nick.get_untracked(), self.color)
	}
}



pub struct ChangeNick( pub String );


impl Message for ChangeNick { type Return = (); }


impl Handler< ChangeNick > for User
{
	#[async_fn_nosend] fn handle_local( &mut self, new_nick: ChangeNick )
	{
		self.set_nick.update( |old| *old = new_nick.0 );
	}
}





