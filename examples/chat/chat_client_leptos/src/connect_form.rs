use crate :: { import::*, * };

const WS_URL         : &str = "ws://127.0.0.1:3012";
const ERR_NICK_LENGTH: &str = "The nick name is invalid. It must be between 1 and 15 word characters.";
const ERR_NICK_IN_USE: &str = "The nick name is already in use. Please choose another.";


#[component]
pub fn ConnectFormDom
(
	cx   : Scope                    ,
	addr : Addr<ConnectForm>        ,
	nick : ReadSignal<&'static str> ,
	url  : ReadSignal<&'static str> ,
	error: ReadSignal<&'static str> ,

) -> impl IntoView
{
	let addr1 = addr.clone();
	let addr2 = addr.clone();

	let on_submit = move |e| connect_submit(e, addr1.clone());
	let on_reset  = move |e| connect_reset (e, addr2.clone());

	let error_display = move || if error.get() == "" { "none" } else { "block" };

	view! { cx,

		// <!-- if we don't put the javascript link, it will still submit when programatorically triggering
		// click on the button... -->

		<form id="connect_form" action="javascript:void(0);" on:submit = on_submit on:reset = on_reset >

			<div id="connect_form_content">

				<p
					style:display = error_display
					id            = "connect_error"
				>
					{ error }
				</p>

				<label for="connect_nick">
					<span>Nickname <span class="required">*</span></span>
					<input type="text" id="connect_nick" name="connect_nick" value= move|| nick.get() />
				</label>

				<label for="connect_url">
					<span>Url <span class="required">*</span></span>
					<input type="text" id="connect_url" name="connect_url" value= move || url.get() />
				</label>

				<div class="buttons">
					<input id="conn_submit" type = "submit" value = "Connect" />
					<input id="conn_reset"  type = "reset"  value = "Reset"   />
				</div>

			</div>

		</form>

	}
}


fn connect_submit(e: web_sys::SubmitEvent, mut conn_form: Addr<ConnectForm> )
{
	Bindgen.spawn_local( async move
	{
		e.prevent_default();

		conn_form.send(ConnSubmitEvt).await.expect_throw( "send ConnSubmitEvt" );

	}).expect_throw( "spawn send ConnSubmitEvt" );
}


fn connect_reset(e: Event, mut conn_form: Addr<ConnectForm> )
{
	Bindgen.spawn_local( async move
	{
		e.prevent_default();

		conn_form.send(ConnResetEvt).await.expect_throw( "send ConnResetEvt" );

	}).expect_throw( "spawn send ConnResetEvt" );
}



#[ derive( Actor ) ]
//
pub struct ConnectForm
{
	app      : Addr< App >               ,
	set_nick : WriteSignal<&'static str> ,
	set_url  : WriteSignal<&'static str> ,
	set_error: WriteSignal<&'static str> ,
}


impl ConnectForm
{
	pub fn new( app: Addr<App>, self_addr: Addr<Self> ) -> Self
	{
		let cx = CX.with( |cx| *cx.get().expect_throw( "cx to be created" ) );

		let (nick, set_nick) = create_signal( cx, random_name() );
		let (url, set_url) = create_signal( cx, "ws://127.0.0.1:3012" );
		let (error, set_error) = create_signal( cx, "" );

		mount_to_global( get_id( "page_login" ), Operation::Append, move |cx|
		{
			view! { cx, <ConnectFormDom addr=self_addr nick=nick url=url error=error /> }
		});

		Self
		{
			app      ,
			set_nick ,
			set_url  ,
			set_error,
		}
	}


	// TODO validate something
	//
	fn validate_connect_form() -> Result< (String, String), () >
	{
		let nick_field: HtmlInputElement = get_id( "connect_nick" ).unchecked_into();
		let url_field : HtmlInputElement = get_id( "connect_url"  ).unchecked_into();

		let nick = nick_field.value();
		let url  = url_field .value();

		Ok((nick, url))
	}
}



#[ derive(Message) ] pub struct ConnSubmitEvt;

impl Handler< ConnSubmitEvt > for ConnectForm
{
	#[async_fn_nosend] fn handle_local( &mut self, _msg: ConnSubmitEvt )
	{
		info!( "Connect button clicked" );

		// validate form
		//
		let (nick, url) = match Self::validate_connect_form()
		{
			Ok(ok) => ok,

			Err(_) =>
			{
				// report error to the user
				// continue loop
				//
				unreachable!()
			}
		};


		let (ws, wsio) = match WsMeta::connect( url, None ).await
		{
			Ok(conn) => conn,

			Err(e)   =>
			{
				// report error to the user
				//
				error!( "{}", e );
				panic!( "{}", e );
			}
		};

		let (mut peer, peer_mb, peer_addr) = CborWF::create_peer
		(
			"server",
			wsio.into_io(),
			10_000_000, // 10 MB
			10_000_000, // 10 MB
			Bindgen,
			None,
			None,
		).expect_throw( "create server peer" );

		let mut server_addr = server_map::RemoteAddr::new( peer_addr.clone() );

		// Create a service map.
		// A service map is a helper object created by a beefy macro included with thespis_remote_impl. It is responsible
		// for deserializing and delivering the message to the correct handler.
		//
		let mut sm = client_map::Services::new();


		// Register our handlers.
		// We just let the App actor handle messages coming in from the server.
		//
		sm.register_handler::< ServerMsg >( self.app.clone_box() );

		// register service map with peer
		// This tells this peer to expose all these services over this connection.
		//
		peer.register_services( Arc::new( sm ) );



		Bindgen.spawn_local( peer_mb.start_local( peer ).map(|_|()) ).expect_throw("spawn server peer");

		debug!( "Trying to join" );

		// Ask the server to join
		// TODO: fix the expect
		//
		match server_addr.call( Join{ nick } ).await.expect_throw( "contact server" )
		{
			Ok( welcome ) =>
			{
				get_id( "page_login" ).style().set_property( "display", "none" ).expect_throw( "set login page display none" );
				get_id( "page_chat" ).style().set_property( "display", "grid" ).expect_throw( "set chat page display grid" );

				let connection = Connected
				{
					peer_addr   ,
					server_addr ,
					welcome     ,
					ws          ,
				};

				self.app.call( connection ).await.expect_throw( "call" );
			}

			Err(e) =>
			{
				error!( "{}", e );

				match e.kind()
				{
					ChatErrKind::NickInvalid => self.set_error.set( ERR_NICK_LENGTH ),
					ChatErrKind::NickInuse   => self.set_error.set( ERR_NICK_IN_USE ),

					_ => unreachable!( "Unknown ChatErrKind" ),
				}
			}
		};
	}
}



#[ derive(Message) ] pub struct ConnResetEvt;

impl Handler< ConnResetEvt > for ConnectForm
{
	#[async_fn_nosend] fn handle_local( &mut self, _msg: ConnResetEvt )
	{
		self.set_nick .set( random_name() );
		self.set_url  .set( WS_URL        );
		self.set_error.set( ""            );
	}
}
