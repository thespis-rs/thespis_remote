use crate::{ import::*, * };

const HELP: &str = "Available commands:
/nick NEWNAME # change nick (must be between 1 and 15 word characters)
/help # Print available commands";





#[component]
pub fn ChatWindowDom( cx: Scope ) -> impl IntoView
{
	view! { cx,

		<div id="chat"></div>
	}
}


#[ derive( Debug, Actor ) ]
//
pub struct ChatWindow
{
	div      : HtmlDivElement               ,
	users    : HashMap< usize, Addr<User> > ,
	srv_color: Color                        ,
	_clt_color: Color                       ,
}


impl ChatWindow
{
	pub fn new( dom_id: &str ) -> Self
	{
		Self
		{
			div       : get_id( dom_id ).unchecked_into(),
			users     : HashMap::new()                   ,
			srv_color : Color::random().light()          ,
			_clt_color : Color::random().light()         ,
		}
	}



	fn append_line( &mut self, time: f64, nick: &str, line: &str, color: &Color, color_all: bool )
	{
		let p: HtmlElement = document().create_element( "p"    ).expect_throw( "create p"    ).unchecked_into();
		let n: HtmlElement = document().create_element( "span" ).expect_throw( "create span" ).unchecked_into();
		let m: HtmlElement = document().create_element( "span" ).expect_throw( "create span" ).unchecked_into();
		let t: HtmlElement = document().create_element( "span" ).expect_throw( "create span" ).unchecked_into();

		n.style().set_property( "color", &color.to_css() ).expect_throw( "set color" );

		if color_all
		{
			m.style().set_property( "color", &color.to_css() ).expect_throw( "set color" );
		}

		// Js needs milliseconds, where the server sends seconds
		//
		let time = Date::new( &( time * 1000.0 ).into() );

		n.set_inner_text( &format!( "{}: ", nick )                                                                     );
		m.set_inner_text( line                                                                                         );
		t.set_inner_text( &format!( "{:02}:{:02}:{:02} - ", time.get_hours(), time.get_minutes(), time.get_seconds() ) );

		n.set_class_name( "nick"         );
		m.set_class_name( "message_text" );
		t.set_class_name( "time"         );

		p.append_child( &t ).expect_throw( "Couldn't append child" );
		p.append_child( &n ).expect_throw( "Couldn't append child" );
		p.append_child( &m ).expect_throw( "Couldn't append child" );

		// order is important here, we need to measure the scroll before adding the item
		//
		let max_scroll = self.div.scroll_height() - self.div.client_height();
		self.div.append_child( &p ).expect_throw( "Coundn't append child" );

		// Check whether we are scolled to the bottom. If so, we autoscroll new messages
		// into vies. If the user has scrolled up, we don't.
		//
		// We keep a margin of up to 2 pixels, because sometimes the two numbers don't align exactly.
		//
		if ( self.div.scroll_top() - max_scroll ).abs() < 3
		{
			p.scroll_into_view();
		}
	}
}



impl Handler<Welcome> for ChatWindow
{
	#[async_fn_local] fn handle_local( &mut self, msg: Welcome )
	{
		// TODO: srv_color get's copied here, which is a bit silly!
		//
		let color = self.srv_color;
		self.append_line( msg.time as f64, "Server", &msg.txt, &color, true );
	}

	#[async_fn] fn handle(&mut self, _: Welcome) { unreachable!("cannot be called multithreaded")}
}


impl Handler< Disconnect > for ChatWindow
{
	fn handle( &mut self, _: Disconnect ) -> Return<()>
	{
		self.div.set_inner_html( "" );
		self.users.clear();

		ready(()).boxed()
	}
}




pub struct ChatMsg
{
	pub time: f64    ,
	pub sid : usize  ,
	pub txt : String ,
}

impl Message for ChatMsg { type Return = (); }


impl Handler<ChatMsg> for ChatWindow
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: ChatMsg )
	{
		// TODO: get rid of expect
		//
		let user = self.users.get_mut( &msg.sid ).expect_throw( "Couldn't find user" );

		let (_sid, nick, color) = user.call( UserInfo{} ).await.expect_throw( "call user" );

		self.append_line( msg.time, &nick, &msg.txt, &color, false );

	}
}



/// A user that joined after we are connected
/// We will print a message to announce the user.
//
pub struct NewUser
{
	pub sid     : usize      ,
	pub nick    : String     ,
	pub addr    : Addr<User> ,
	pub time    : f64        ,
}

impl Message for NewUser { type Return = (); }

impl Handler<NewUser> for ChatWindow
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: NewUser )
	{
		self.users.insert( msg.sid, msg.addr );

		if msg.time != 0.0
		{
			let color = self.srv_color;

			self.append_line( msg.time, "Server", &format!( "We welcome a new user, {}!", &msg.nick ), &color, true );
		}
	}
}



pub struct UserLeft { pub time: f64, pub sid: usize }

impl Message for UserLeft { type Return = (); }

impl Handler<UserLeft> for ChatWindow
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: UserLeft )
	{
		// TODO: get rid of expect
		//
		let user = self.users.get_mut( &msg.sid ).expect_throw( "ChatWindow - UserLeft: Couldn't find user" );

		let (_sid, nick, _color) = user.call( UserInfo{} ).await.expect_throw( "call user" );

		self.users.remove( &msg.sid );

		let color = self.srv_color;

		self.append_line( msg.time, "Server", &format!( "Sadly, {} left us.", &nick ), &color, true );

	}
}




pub struct AnnounceNick
{
	pub time: f64    ,
	pub old : String ,
	pub new : String ,
}

impl Message for AnnounceNick { type Return = (); }

impl Handler<AnnounceNick> for ChatWindow
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: AnnounceNick )
	{
		// TODO: srv_color get's copied here, which is a bit silly!
		//
		let color = self.srv_color;

		let txt = format!( "{} has changed names => {}.", &msg.old, &msg.new );

		self.append_line( msg.time, "Server", &txt, &color, true );
	}
}




pub struct PrintHelp {}

impl Message for PrintHelp { type Return = (); }

impl Handler<PrintHelp> for ChatWindow
{
	#[async_fn_nosend] fn handle_local( &mut self, _: PrintHelp )
	{
		let color = self.srv_color;

		self.append_line( Date::now(), "Client", HELP, &color, true );
	}
}
