use crate :: { import::*, * };




#[component]
pub fn UserListDom( cx: Scope, users: ReadSignal<HashMap<usize, String>> ) -> impl IntoView
{
	// let count = create_memo( cx, move |_| users.len() );

	// let add_user = move |_| {
	// 	// let (name, set_name) = create_signal( cx, "" );
	// 	// let color = format!( "--user-color-{}", id );
	// 	// let (color, _set_color) = use_css_var(cx, varname.clone() );

	//     // create a signal for the new counter
	//     let user = create_signal(cx, next_counter_id + 1);

	//     set_users.update(move |usersers| {
	//         users.insert((next_counter_id, user));
	//     });
	// };

	view! { cx,

		<div id="users">
			<p>Total users: count</p>
			<For
				each = move || users.get()
				key  = |(id, _name)| *id
				view = |cx, (_id, name)|
				{
					view!{ cx,
						<p style="color: red">{name}</p>
					}
				}
			/>
		</div>
	}
}


struct UserData
{
	name_read: ReadSignal<String>,
	name_write: WriteSignal<String>,
}


#[ derive( Actor ) ]
//
pub struct UserList
{
	// users       : HashMap::<usize, Addr<User>>,
	set_users   : WriteSignal<HashMap::<usize, String>>,
	indom       : bool                        ,
	chat_window : Addr< ChatWindow >          ,
}

impl UserList
{
	pub fn new( chat_window: Addr< ChatWindow >, set_users: WriteSignal<HashMap::<usize, String>>, cx: Scope ) -> Self
	{
		Self
		{
			chat_window,
			set_users,
			indom: false,
		}
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

			let update = user_count::Update{ count: self.users.len() };
			self.user_count.send( update ).await.expect_throw( "send user count update" );

			addr
		}
	}

	#[async_fn] fn handle(&mut self, _: Insert) -> Addr<User> { unreachable!("cannot be called multithreaded")}
}



pub struct Remove { pub sid: usize }

impl Message for Remove { type Return = (); }


impl Handler< Remove > for UserList
{
	#[async_fn_local] fn handle_local( &mut self, msg: Remove )
	{
		self.users.remove( &msg.sid );

		let update = user_count::Update{ count: self.users.len() };
		self.user_count.send( update ).await.expect_throw( "send user count update" );
	}

	#[async_fn] fn handle(&mut self, _: Remove) { unreachable!("cannot be called multithreaded")}
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
	#[async_fn_local] fn handle_local( &mut self, msg: Render )
	{
		let _ = create_signal( self.cx, []);

		leptos::mount_to( document().body().unwrap(), |_|
		{
			let (view, disposer) = self.cx.run_child_scope(
				|cx| view!{ cx, <p>bla</p> }
			);

			view
		});

		for user in self.users.values_mut()
		{
			user.send( msg ).await.expect_throw( "send" );
		}

		self.user_count.send( msg ).await.expect_throw( "render user_count" );


		if !self.indom
		{
			self.parent.append_child( &self.div ).expect_throw( "add udiv to dom" );

			self.indom = true;
		}
	}


	#[async_fn] fn handle(&mut self, _: Render) { unreachable!("cannot be called multithreaded")}
}




pub struct GetUser { pub sid: usize }

impl Message for GetUser { type Return = Addr<User>; }


impl Handler< GetUser > for UserList
{
	#[async_fn_local] fn handle_local( &mut self, msg: GetUser ) -> Addr<User>
	{
		// TODO: get rid of expect
		//
		self.users.get( &msg.sid ).expect_throw( "user" ).clone()
	}

	#[async_fn] fn handle(&mut self, _: GetUser) -> Addr<User> { unreachable!("cannot be called multithreaded")}
}



