use crate :: { import::*, * };


#[ derive( Actor ) ]
//
pub struct UserList
{
	users       : HashMap<usize, Addr<User>>,
	div         : HtmlDivElement            ,
	indom       : bool                      ,
	parent      : HtmlElement               ,
	chat_window : Addr< ChatWindow >        ,
}

// Unfortunately thespis requires Send right now, and HtmlElement isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for UserList {}



impl UserList
{
	pub fn new( parent: &str, chat_window: Addr< ChatWindow > ) -> Self
	{
		Self
		{
			chat_window,
			users: HashMap::new() ,
			div  : document().create_element( "div" ).expect_throw( "create userlist div" ).unchecked_into() ,
			indom: false,
			parent: get_id( parent ).unchecked_into(),
		}
	}
}


impl Drop for UserList
{
	fn drop( &mut self )
	{
		// remove self from Dom
		//
		self.div.remove();
		//
		// Delete children
		//

	}
}


pub struct Insert
{
	pub sid    : usize  ,
	pub nick   : String ,
	pub time   : f64    ,
	pub is_self: bool   ,
}


impl Message for Insert { type Return = Addr<User>; }


impl Handler< Insert > for UserList
{
	fn handle( &mut self, msg: Insert ) -> Return< Addr<User> > { Box::pin( async move
	{
		let _render = false;

		let users = &mut self.users;

		if let Some( user ) = users.get_mut( &msg.sid )
		{
			user.send( ChangeNick(msg.nick.clone()) ).await.expect_throw( "send" );

			return user.clone();
		}

		else
		{
			let user = User::new( msg.sid, msg.nick.clone(), self.div.clone().unchecked_into(), msg.is_self );
			let mut addr = Addr::builder("user").spawn_local( user, &Bindgen ).expect_throw( "spawn user" );

			addr.send( Render{} ).await.expect_throw( "send" );

			self.users.insert( msg.sid, addr.clone() );

			let new_user = NewUser
			{
				time: msg.time     ,
				nick: msg.nick     ,
				sid : msg.sid      ,
				addr: addr.clone() ,
			};

			self.chat_window.send( new_user ).await.expect_throw( "send" );

			addr
		}
	})}
}



pub struct Remove { pub sid: usize }

impl Message for Remove { type Return = (); }


impl Handler< Remove > for UserList
{
	#[async_fn_local] fn handle_local( &mut self, msg: Remove )
	{
		self.users.remove( &msg.sid );
	}

	#[async_fn] fn handle(&mut self, _: Remove) { unreachable!("cannot be called multithreaded")}
}



pub struct Clear {}

impl Message for Clear { type Return = (); }


impl Handler< Clear > for UserList
{
	fn handle( &mut self, _: Clear ) -> Return<()> { Box::pin( async move
	{
		self.users.clear();
	})}
}




pub struct Render {}

impl Message for Render { type Return = (); }


impl Handler< Render > for UserList
{
	fn handle( &mut self, _: Render ) -> Return<()> { Box::pin( async move
	{
		for user in self.users.values_mut()
		{
			user.send( Render{} ).await.expect_throw( "send" );
		}

		if !self.indom
		{
			self.parent.append_child( &self.div ).expect_throw( "add udiv to dom" );

			self.indom = true;
		}

	})}
}




pub struct GetUser { pub sid: usize }

impl Message for GetUser { type Return = Addr<User>; }


impl Handler< GetUser > for UserList
{
	fn handle( &mut self, msg: GetUser ) -> Return< Addr<User> > { Box::pin( async move
	{
		// TODO: get rid of expect
		//
		self.users.get( &msg.sid ).expect_throw( "user" ).clone()

	})}
}



