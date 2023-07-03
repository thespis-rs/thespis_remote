use crate::{ import::*, User };

#[ derive( Actor ) ]
//
pub struct Server
{
	users: HashMap< usize, UserData >
}



impl Server
{
	const WELCOME : &'static str = "Welcome to the thespis Chat Server!" ;

	pub fn new() -> Self
	{
		Server
		{
			users: HashMap::new(),
		}
	}



	async fn broadcast( &mut self, msg: ServerMsg )
	{
		for user in self.users.values_mut()
		{
			user.peer_addr.send( msg.clone() ).await.expect( "broadcast" );
		};
	}


	fn validate_nick( &self, new: &str, old: Option<&str> ) -> Result<(), ChatErr>
	{
		// Check whether it's unchanged
		//
		if let Some(o) = old {
		if o == new
		{
			return Err( ChatErrKind::NickUnchanged.into() );
		}}


		// Check whether it's not already in use
		//
		if self.users.values().any( |c| c.nick == new )
		{
			return Err( ChatErrKind::NickInuse.into() )
		}


		// Check whether it's valid
		//
		let nickre = Regex::new( r"^\w{1,15}$" ).unwrap();

		if !nickre.is_match( new )
		{
			error!( "Invalid nick: '{}'", new );
			return Err( ChatErrKind::NickInvalid.into() )
		}


		// It's valid
		//
		// Ok( ServerMsg::NickChanged{ time: Utc::now().timestamp(), old: old.to_string(), new: new.to_string(), sid } )
		Ok(())
	}
}


pub struct UserData
{
	pub nick     : String                                       ,
	pub addr     : Addr<User>                                   ,
	pub peer_addr: client_map::RemoteAddr,

}

impl Message for UserData { type Return = Result<Welcome, ChatErr>; }


impl Handler< UserData > for Server
{
	#[async_fn] fn handle( &mut self, new_user: UserData ) -> Result<Welcome, ChatErr>
	{
		let validate = self.validate_nick( &new_user.nick, None );

		match validate
		{
			Ok(_) =>
			{
				let id = new_user.addr.id();

				let joined = ServerMsg::UserJoined
				{
					sid : id,
					nick: new_user.nick.clone(),
					time: Utc::now().timestamp(),
				};

				self.broadcast( joined ).await;


				let user_data = UserData{ addr: new_user.addr, nick: new_user.nick, peer_addr: new_user.peer_addr };

				let opt = self.users.insert( id, user_data );


				let welcome = Welcome
				{
					sid  : id,
					txt  : Self::WELCOME.to_string(),
					users: self.users.values().map( |ud| (ud.addr.id(), ud.nick.clone()) ).collect(),
					time : Utc::now().timestamp(),
				};

				// A connection shouldn't try to create 2 users, so we verify it didn't exist yet,
				// since this is a programmer error in this very app, just panic
				//
				assert!( opt.is_none() );

				Ok( welcome )
			}

			Err(e) =>
			{
				Err( e )
			}
		}
	}
}




pub struct ChangeNick
{
	pub sid : usize ,
	pub nick: String,
}


impl Message for ChangeNick { type Return = Result<(), ChatErr>; }


impl Handler< ChangeNick > for Server
{
	fn handle( &mut self, change_nick: ChangeNick ) -> Return< Result<(), ChatErr> > { Box::pin( async move
	{
		let mut user_data = self.users.remove( &change_nick.sid ).expect( "join first" );
		let validate = self.validate_nick( &change_nick.nick, Some( &user_data.nick.clone() ) );

		match validate
		{
			Ok(_) =>
			{
				let changed = ServerMsg::NickChanged
				{
					sid : change_nick.sid,
					nick: change_nick.nick.clone(),
					time: Utc::now().timestamp(),
				};

				user_data.nick = change_nick.nick;

				self.users.insert( change_nick.sid, user_data );

				self.broadcast( changed ).await;

				Ok(())
			}

			Err(e) =>
			{
				self.users.insert( change_nick.sid, user_data );

				Err( e )
			}
		}

	})}
}




pub struct ChatMsgIn
{
	pub sid: usize ,
	pub txt: String,
}


impl Message for ChatMsgIn { type Return = (); }


impl Handler< ChatMsgIn > for Server
{
	fn handle( &mut self, msg: ChatMsgIn ) -> Return<()> { Box::pin( async move
	{
		let smsg = ServerMsg::ChatMsg
		{
			time: Utc::now().timestamp() ,
			sid : msg.sid                ,
			txt : msg.txt                ,
		};

		self.broadcast( smsg ).await;

	})}
}


pub struct ConnectionClosed( pub usize );


impl Message for ConnectionClosed { type Return = (); }


impl Handler< ConnectionClosed > for Server
{
	fn handle( &mut self, msg: ConnectionClosed ) -> Return<()> { Box::pin( async move
	{
		let _ = self.users.remove( &msg.0 );

		let smsg = ServerMsg::UserLeft
		{
			time: Utc::now().timestamp() ,
			sid : msg.0                  ,
		};

		self.broadcast( smsg ).await;
	})}
}
