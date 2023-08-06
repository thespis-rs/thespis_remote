use crate :: { import::*, * };




#[component]
pub fn UserListDom( cx: Scope, count: ReadSignal<usize> ) -> impl IntoView
{
	view! { cx,

		<div id="user_list">
			<div id="user_count">{"Total users: "}{count}</div>
			<ul id="user_list_ul"></ul>
		</div>
	}
}


#[ derive( Actor ) ]
//
pub struct UserList
{
	users       : HashMap< usize, Addr<User> >,
	chat_window : Addr< ChatWindow >          ,
	set_count   : WriteSignal< usize >        ,
	ul          : HtmlElement                 ,
}

impl UserList
{
	pub fn new( parent: &str, chat_window: Addr< ChatWindow > ) -> Self
	{
		let parent = get_id( parent );

		let cx = CX.with( |cx| *cx.get().expect_throw( "cx to be created" ) );
		let (count, set_count) = create_signal(cx, 0);

		mount_to_global( parent.clone(), Operation::Append, move |cx|
		{
			view! { cx, <UserListDom count=count /> }
		});

		let ul = get_id( "user_list_ul" );

		Self
		{
			chat_window,
			users: HashMap::new() ,
			set_count,
			ul
		}
	}

	fn update_count( &self )
	{
		self.set_count.update( |old| *old = self.users.len() );
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
	#[async_fn_nosend] fn handle_local( &mut self, msg: Insert ) -> Addr<User>
	{
		let users = &mut self.users;

		if let Some( user ) = users.get_mut( &msg.sid )
		{
			user.send( ChangeNick(msg.nick.clone()) ).await.expect_throw( "send" );

			return user.clone();
		}

		else
		{
			let user = User::new( msg.sid, msg.nick.clone(), self.ul.clone(), msg.is_self );
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



