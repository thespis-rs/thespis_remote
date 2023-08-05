use crate::{ *, import::* };


// #[component]
// pub fn AppDom( cx: Scope ) -> impl IntoView
// {
// 	view! { cx,



// 		<ChatWindowDom />
// 		<UserListDom />
// 		<ChatFormDom addr=chat_form />
// 		<ConnectFormDom addr=conn_form />
// 	}
// }


// The central logic of our application. This will receive the incoming messages from the server and
// dispatch the information to all components that need it.
//
#[ derive( Debug, Actor ) ]
//
pub struct App
{
	connection : Option< Connected >,
	chat_window: Addr< ChatWindow > ,
	user_list      : Addr< UserList   > ,
}


impl App
{
	pub fn new( app: Addr<Self> ) -> Self
	{


		let chat_window = Addr::builder("chat_window")
			.spawn_local( ChatWindow::new("chat"), &Bindgen )
			.expect_throw( "spawn chat_window"  )
		;

		let user_list = Addr::builder("user_list")
			.spawn_local( UserList::new( "user_list", chat_window.clone() ), &Bindgen )
			.expect_throw( "spawn userlist"  )
		;

		let (conn_form_addr, conn_form_mb) = Addr::builder( "conn_form" ).build();
		let conn_form = ConnectForm::new( app.clone(), conn_form_addr );

		Bindgen.spawn_local( async{ conn_form_mb.start_local( conn_form ).await; } )
			.expect_throw( "spawn conn_form" )
		;

		let (chat_form_addr, chat_form_mb) = Addr::builder( "chat_form" ).build();
		let chat_form = ChatForm::new( app, chat_window.clone(), chat_form_addr );

		Bindgen.spawn_local( async{ chat_form_mb.start_local( chat_form ).await; } )
			.expect_throw( "spawn chat_form" )
		;


		Self
		{
			chat_window      ,
			user_list            ,
			connection: None ,
		}
	}
}



// When we are successfully connected, store the peer that handles the connection.
//
#[ derive( Debug ) ]
//
pub struct Connected
{
	pub peer_addr   : WeakAddr<Peer>         ,
	pub server_addr : server_map::RemoteAddr ,
	pub welcome     : Welcome                ,
	pub ws          : WsMeta                 ,
}

unsafe impl Send for Connected {}

impl Message for Connected { type Return = (); }


impl Handler<Connected> for App
{
	fn handle_local( &mut self, mut msg: Connected ) -> ReturnNoSend<()> { Box::pin( async move
	{
		self.user_list.call( Clear{} ).await.expect_throw( "clear userlist" );

		let new_users = std::mem::take(&mut msg.welcome.users);

		// A time of 0.0 means that the user joined before us, so we don't print an annoucnement.
		//
		let inserts = new_users.into_iter().map( |(sid, nick)| Ok(Insert{ time: 0.0, sid, nick, is_self: sid == msg.welcome.sid }) );

		warn!( "{:?}", &inserts );

		self.user_list.send_all( &mut futures::stream::iter(inserts) ).await.expect_throw( "new users" );

		self.user_list.send( Render{} ).await.expect_throw( "render userlist" );

		self.chat_window.call( msg.welcome.clone() ).await.expect_throw( "call" );

		self.connection = Some( msg );

	})}



	fn handle( &mut self, _: Connected ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}



impl Handler<ServerMsg> for App
{
	#[async_fn_local] fn handle_local( &mut self, msg: ServerMsg )
	{
		match msg
		{
			// Add the user to the userlist
			// Give the chat window this users Addr
			//
			ServerMsg::UserJoined{ time, sid, nick } =>
			{
				debug!( "ServerMsg::UserJoined" );
				self.user_list.send( Insert{ time: time as f64, sid, nick, is_self: false } ).await

					.expect_throw( "add new user to user list" )
				;
			}

			// Remove the user to the userlist
			// Notify the chat window so it removes the user from it's list.
			//
			ServerMsg::UserLeft{ time, sid } =>
			{
				debug!( "ServerMsg::UserLeft" );

				self.user_list.call( Remove{ sid } ).await.expect_throw( "remove user from user list" );

				self.chat_window.send( UserLeft{ time: time as f64, sid } ).await.expect_throw( "announce new user" );
			}

			// Inform the correct user
			// Let chat window know to print a message.
			//
			ServerMsg::NickChanged{ sid, time, nick }  =>
			{
				debug!( "ServerMsg::NickChanged" );

				let mut user = self.user_list.call( GetUser{ sid } ).await.expect_throw( "get user info" );

				let (old, _) = user.call( UserInfo {} ).await.expect_throw( "get user info" );
				user.call( ChangeNick( nick.clone() ) ).await.expect_throw( "change nick" );

				self.chat_window.send( AnnounceNick{ time: time as f64, old, new: nick } ).await.expect_throw( "announce nick" );
			}

			// Print the message to the chat window
			//
			ServerMsg::ChatMsg{ time, sid, txt } =>
			{
				debug!( "ServerMsg::ChatMsg" );

				self.chat_window.send( ChatMsg{ time: time as f64, sid, txt } ).await.expect_throw( "announce nick" );
			}
		}
	}



	fn handle( &mut self, _: ServerMsg ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}



// Inform the correct user
//
impl Handler<NewChatMsg> for App
{
	fn handle_local( &mut self, msg: NewChatMsg ) -> ReturnNoSend<()> { Box::pin( async move
	{
		let conn = self.connection.as_mut().expect_throw( "be connected" );

		conn.server_addr.send( msg ).await.expect_throw( "send new chat message" );

	})}


	fn handle( &mut self, _: NewChatMsg ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}



// Inform the correct user
//
impl Handler<SetNick> for App
{
	#[async_fn_local] fn handle_local( &mut self, msg: SetNick ) -> Result<(), ChatErr>
	{
		let conn = self.connection.as_mut().expect_throw( "be connected" );

		conn.server_addr.call( msg ).await.expect_throw( "forward SetNick" )
	}


	#[async_fn] fn handle( &mut self, _: SetNick ) -> Result<(), ChatErr>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}


pub struct Disconnect {}

impl Message for Disconnect { type Return = (); }

// Inform the correct user
//
impl Handler<Disconnect> for App
{
	#[async_fn_nosend] fn handle_local( &mut self, _: Disconnect )
	{
		if let Some( conn ) = &self.connection
		{
			conn.ws.close().await.expect_throw( "close connection" );
		}

		self.connection = None;

		// show the connect form
		//
		let cform: HtmlElement = get_id( "connect_form" ).unchecked_into();

		cform.style().set_property( "display", "flex" ).expect_throw( "set cform display none" );

		self.user_list.send( Clear {} ).await.expect_throw( "clear  userlist" );
		self.user_list.send( Render{} ).await.expect_throw( "render userlist" );

		self.chat_window.send( Disconnect{} ).await.expect_throw( "clear  userlist" );

	}
}
