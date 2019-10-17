use crate :: { import::*, color::*, document, user_list::Render };


#[ derive( Actor ) ]
//
pub struct User
{
	sid   : usize                ,
	nick  : String               ,
	color : Color                ,
	p     : HtmlParagraphElement ,
	indom : bool                 ,
	parent: HtmlElement          ,
}

// Unfortunately thespis requires Send right now, and HtmlElement isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for User {}

impl User
{
	pub fn new( sid: usize, nick: String, parent: HtmlElement ) -> Self
	{
		Self
		{
			nick  ,
			sid   ,
			parent,
			color: Color::random().light(),
			p    : document().create_element( "p" ).expect_throw( "create user p" ).unchecked_into(),
			indom: false,
		}
	}


	pub fn render( &mut self )
	{
		self.p.set_inner_text( &self.nick );

		self.p.style().set_property( "color", &self.color.to_css() ).expect_throw( "set color" );
		self.p.set_id( &format!( "user_{}", &self.sid ) );

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
	fn handle( &mut self, _: UserInfo ) -> Return<(String, Color)> { Box::pin( async move
	{
		(self.nick.clone(), self.color)

	})}
}



pub struct ChangeNick( pub String );


impl Message for ChangeNick { type Return = (); }


impl Handler< ChangeNick > for User
{
	fn handle( &mut self, new_nick: ChangeNick ) -> Return<()> { Box::pin( async move
	{
		self.nick = new_nick.0;

		if self.indom
		{
			self.render()
		}

	})}
}


impl Handler< Render > for User
{
	fn handle( &mut self, _: Render ) -> Return<()> { Box::pin( async move
	{
		self.render();

	})}
}





