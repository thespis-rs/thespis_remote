use crate :: { import::*, * };




#[component]
pub fn UserListDom( cx: Scope, count: ReadSignal<usize> ) -> impl IntoView
{
	view! { cx,

		<div id="users">
			<div id="user_count">Total users: {count}</div>
		</div>
	}
}


#[ derive( Actor ) ]
//
pub struct UserList
{
	users       : HashMap< usize, Addr<User> >,
	div         : HtmlDivElement              ,
	indom       : bool                        ,
	parent      : HtmlElement                 ,
	chat_window : Addr< ChatWindow >          ,
	set_count   : WriteSignal< usize >        ,
}

impl UserList
{
	pub fn new( parent: &str, chat_window: Addr< ChatWindow > ) -> Self
	{
		let parent = get_id( parent );

		let cx = CX.with( |cx| *cx.get().expect_throw( "cx to be created" ) );
		let (count, set_count) = create_signal(cx, 0);

		mount_to_global( parent.clone(), move |cx|
		{
			view! { cx, <UserListDom count=count /> }
		});

		Self
		{
			chat_window,
			users: HashMap::new() ,
			div  : document().create_element( "div" ).expect_throw( "create userlist div" ).unchecked_into() ,
			indom: false,
			parent,
			set_count,
		}
	}

	fn update_count( &self )
	{
		self.set_count.update( |old| *old = self.users.len() );
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
	#[async_fn_local] fn handle_local( &mut self, msg: Insert ) -> Addr<User>
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
			let addr = Addr::builder("user").spawn_local( user, &Bindgen ).expect_throw( "spawn user" );

			self.users.insert( msg.sid, addr.clone() );

			let new_user = NewUser
			{
				time: msg.time     ,
				nick: msg.nick     ,
				sid : msg.sid      ,
				addr: addr.clone() ,
			};

			self.chat_window.send( new_user ).await.expect_throw( "send" );
			self.update_count();

			addr
		}
	}

	#[async_fn] fn handle(&mut self, _: Insert) -> Addr<User> { unreachable!("cannot be called multithreaded")}
}



pub struct Remove { pub sid: usize }

impl Message for Remove { type Return = (); }


impl Handler< Remove > for UserList
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: Remove )
	{
		self.users.remove( &msg.sid );
		self.update_count();
	}
}



pub struct Clear {}

impl Message for Clear { type Return = (); }


impl Handler< Clear > for UserList
{
	#[async_fn_local] fn handle_local( &mut self, _: Clear )
	{
		self.users.clear();
	}

	#[async_fn] fn handle(&mut self, _: Clear) { unreachable!("cannot be called multithreaded")}
}


#[derive(Clone, Copy)]
pub struct Render;

impl Message for Render { type Return = (); }


impl Handler< Render > for UserList
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: Render )
	{
		for user in self.users.values_mut()
		{
			user.send( msg ).await.expect_throw( "send" );
		}

		if !self.indom
		{
			self.parent.append_child( &self.div ).expect_throw( "add udiv to dom" );

			self.indom = true;
		}
	}
}




pub struct GetUser { pub sid: usize }

impl Message for GetUser { type Return = Addr<User>; }


impl Handler< GetUser > for UserList
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: GetUser ) -> Addr<User>
	{
		// TODO: get rid of expect
		//
		self.users.get( &msg.sid ).expect_throw( "user" ).clone()
	}
}



