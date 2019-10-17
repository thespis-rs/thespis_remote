use crate::{ *, import::* };


// The central logic of our application. This will receive the incoming messages from the server and
// dispatch the information to all components that need it.
//
#[ derive( Debug, Actor ) ]
//
pub struct App
{
	connection : Option< Connected >,
	addr       : Addr<Self>         ,
	chat_window: Addr< ChatWindow > ,
	users      : Addr< UserList   > ,
}


impl App
{
	pub fn new( addr: Addr<Self> ) -> Self
	{
		let chat_window = Addr::try_from_local( ChatWindow ::new( "chat"                       ) ).expect_throw( "spawn cwindow"  );
		let users       = Addr::try_from_local( UserList   ::new( "users", chat_window.clone() ) ).expect_throw( "spawn userlist" );

		let app = Self
		{
			addr             ,
			chat_window      ,
			users            ,
			connection: None ,
		};

		app.setup_forms();
		app
	}


	// Link submit and reset buttons to the Actors that will handle the events
	//
	fn setup_forms( &self )
	{
		let conn_form = Addr::try_from_local( ConnectForm::new( self.addr.clone() ) )

			.expect_throw( "spawn conn_form" )
		;

		let chat_form = Addr::try_from_local( ChatForm::new( self.addr.clone(), self.chat_window.clone() ) )

			.expect_throw( "spawn chat_form" )
		;

		let conn_form2 = conn_form.clone();
		let chat_form2 = chat_form.clone();
		let chat_form3 = chat_form.clone();

		let chat_input_elem = get_id( "chat_input"   );
		let conn_form_elem  = get_id( "connect_form" );
		let chat_form_elem  = get_id( "chat_form"    );

		let conn_submit_evts = EHandler::new( &conn_form_elem, "submit", false );
		let conn_reset_evts  = EHandler::new( &conn_form_elem, "reset" , false );

		let chat_submit_evts = EHandler::new( &chat_form_elem, "submit", false );
		let chat_reset_evts  = EHandler::new( &chat_form_elem, "reset" , false );

		let enter_evts       = EHandler::new( &chat_input_elem, "keypress", false );


		// Connect the events from the connect form to the actor handling them. (submit and reset).
		//
		let conn_submit_task = async move
		{
			conn_submit_evts

				.map( |e| Ok(ConnSubmitEvt{e}) )
				.forward( conn_form ).await
				.expect_throw( "forward csubmit" )
			;
		};

		let conn_reset_task = async move
		{
			conn_reset_evts

				.map( |e| Ok(ConnResetEvt{e}) )
				.forward( conn_form2 ).await
				.expect_throw( "forward csubmit" )
			;
		};

		// Connect the events from the caht form to the actor handling them. (submit and disconnect).
		//
		let chat_submit_task = async move
		{
			chat_submit_evts

				.map( |e| Ok(ChatSubmitEvt{e}) )
				.forward( chat_form ).await
				.expect_throw( "forward csubmit" )
			;
		};

		let chat_reset_task = async move
		{
			chat_reset_evts

				.map( |e| Ok(ChatResetEvt{e}) )
				.forward( chat_form2 ).await
				.expect_throw( "forward csubmit" )
			;
		};

		let chat_enter_task = async move
		{
			enter_evts

				.map( |e| Ok(ChatSubmitEvt{e}) )
				.forward( chat_form3 ).await
				.expect_throw( "forward csubmit" )
			;
		};

		spawn_local( conn_submit_task );
		spawn_local( conn_reset_task  );

		spawn_local( chat_enter_task  );
		spawn_local( chat_submit_task );
		spawn_local( chat_reset_task  );
	}
}



// When we are successfully connected, store the peer that handles the connection.
//
#[ derive( Debug ) ]
//
pub struct Connected
{
	pub peer_addr     : Addr<Peer<MS>>           ,
	pub set_nick_addr : BoxRecipient<SetNick>    ,
	pub new_msg_addr  : BoxRecipient<NewChatMsg> ,
	pub welcome       : Welcome                  ,
	pub ws            : WsStream                 ,
}

unsafe impl Send for Connected {}

impl Message for Connected { type Return = (); }


impl Handler<Connected> for App
{
	fn handle_local( &mut self, mut msg: Connected ) -> ReturnNoSend<()> { Box::pin( async move
	{
		self.users.call( Clear{} ).await.expect_throw( "clear userlist" );

		let new_users = std::mem::replace( &mut msg.welcome.users, Vec::new() );

		// A time of 0.0 means that the user joined before us, so we don't print an annoucnement.
		//
		let inserts = new_users.into_iter().map( |(sid, nick)| Insert{ time: 0.0, sid, nick } );

		warn!( "{:?}", &inserts );

		self.users.send_all( &mut futures::stream::iter(inserts) ).await.expect_throw( "new users" );

		self.users.send( Render{} ).await.expect_throw( "render userlist" );

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
	fn handle_local( &mut self, msg: ServerMsg ) -> ReturnNoSend<()> { Box::pin( async move
	{
		match msg
		{
			// Add the user to the userlist
			// Give the chat window this users Addr
			//
			ServerMsg::UserJoined{ time, sid, nick } =>
			{
				self.users.send( Insert{ time: time as f64, sid, nick } ).await

					.expect_throw( "add new user to user list" )
				;
			}

			// Remove the user to the userlist
			// Notify the chat window so it removes the user from it's list.
			//
			ServerMsg::UserLeft{ time, sid } =>
			{
				self.users.call( Remove{ sid } ).await.expect_throw( "remove user from user list" );

				self.chat_window.send( UserLeft{ time: time as f64, sid } ).await.expect_throw( "announce new user" );
			}

			// Inform the correct user
			// Let chat window know to print a message.
			//
			ServerMsg::NickChanged{ sid, time, nick }  =>
			{
				let mut user = self.users.call( GetUser{ sid } ).await.expect_throw( "get user info" );

				let (old, _) = user.call( UserInfo {} ).await.expect_throw( "get user info" );
				user.call( ChangeNick( nick.clone() ) ).await.expect_throw( "change nick" );

				self.chat_window.send( AnnounceNick{ time: time as f64, old, new: nick } ).await.expect_throw( "announce nick" );
			}

			// Print the message to the chat window
			//
			ServerMsg::ChatMsg{ time, sid, txt } =>
			{
				self.chat_window.send( ChatMsg{ time: time as f64, sid, txt } ).await.expect_throw( "announce nick" );
			}
		}

	})}



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

		conn.new_msg_addr.send( msg ).await.expect_throw( "send new chat message" );

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
	fn handle_local( &mut self, msg: SetNick ) -> ReturnNoSend< Result<(), ChatErr> > { Box::pin( async move
	{
		let conn = self.connection.as_mut().expect_throw( "be connected" );

		conn.set_nick_addr.call( msg ).await.expect_throw( "forward SetNick" )
	})}


	fn handle( &mut self, _: SetNick ) -> Return< Result<(), ChatErr> >
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
	fn handle_local( &mut self, _: Disconnect ) -> ReturnNoSend<()> { Box::pin( async move
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

		self.users.send( Clear {} ).await.expect_throw( "clear  userlist" );
		self.users.send( Render{} ).await.expect_throw( "render userlist" );

		self.chat_window.send( Disconnect{} ).await.expect_throw( "clear  userlist" );

	})}


	fn handle( &mut self, _: Disconnect ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}
