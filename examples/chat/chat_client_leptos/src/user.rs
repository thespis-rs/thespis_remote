use crate :: { import::*, color::*, document, user_list::Render };


#[component]
pub fn UserDom( cx: Scope, style: &'static str, name: ReadSignal<String>  ) -> impl IntoView
{
	view! { cx,

		<p style=style>{name}</p>
	}
}



#[ derive( Actor ) ]
//
pub struct User
{
	sid    : usize                ,
	nick   : String               ,
	color  : Color                ,
	p      : HtmlParagraphElement ,
	indom  : bool                 ,
	parent : HtmlElement          ,
	is_self: bool                 ,
}

// Unfortunately thespis requires Send right now, and HtmlElement isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for User {}

impl User
{
	pub fn new( sid: usize, nick: String, parent: HtmlElement, is_self: bool ) -> Self
	{
		Self
		{
			nick  ,
			sid   ,
			parent,
			color: Color::random().light(),
			p    : document().create_element( "p" ).expect_throw( "create user p" ).unchecked_into(),
			indom: false,
			is_self,
		}
	}


	pub fn render( &mut self )
	{
		self.p.set_inner_text( &self.nick );

		self.p.style().set_property( "color", &self.color.to_css() ).expect_throw( "set color" );
		self.p.set_id( &format!( "user_{}", &self.sid ) );

		if self.is_self
		{
			self.p.style().set_property( "font-weight", "bold" ).expect_throw( "set color" );
		}

		if !self.indom
		{
			self.parent.append_child( &self.p ).expect_throw( "append user to div" );
			self.indom = true;
		}
	}
}



impl Drop for User
{
	fn drop( &mut self )
	{
		warn!( "removing user {:?} from dom", self.nick );
		self.p.remove();
	}
}




pub struct UserInfo {}


impl Message for UserInfo { type Return = (String, Color); }


impl Handler< UserInfo > for User
{
	#[async_fn] fn handle( &mut self, _: UserInfo ) -> (String, Color)
	{
		(self.nick.clone(), self.color)
	}
}



pub struct ChangeNick( pub String );


impl Message for ChangeNick { type Return = (); }


impl Handler< ChangeNick > for User
{
	#[async_fn] fn handle( &mut self, new_nick: ChangeNick )
	{
		self.nick = new_nick.0;
		self.p.set_inner_text( &self.nick );
	}
}


impl Handler< Render > for User
{
	#[async_fn] fn handle( &mut self, _: Render )
	{
		self.render();
	}
}





