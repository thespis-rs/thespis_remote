use crate :: { import::*, * };


#[component]
pub fn ConnectFormDom( cx: Scope ) -> impl IntoView
{
	view! { cx,

		// <!-- if we don't put the javascript link, it will still submit when programatorically triggering
		// click on the button... -->

		<form id="connect_form" action="javascript:void(0);" >

			<div id="connect_form_content">

				<p id="connect_error"></p>

				<label for="connect_nick">
					<span>Nickname <span class="required">*</span></span>
					<input type="text" id="connect_nick" name="connect_nick" value="" />
				</label>

				<label for="connect_url">
					<span>Url <span class="required">*</span></span>
					<input type="text" id="connect_url" name="connect_url" value="ws://127.0.0.1:3012" />
				</label>

				<label>
					<span></span>
					<input id="conn_submit" type = "submit" value = "Connect" />
					<input id="conn_reset"  type = "reset"  value = "Reset"   />
				</label>

			</div>

		</form>

	}
}



#[ derive( Actor ) ]
//
pub struct ConnectForm
{
	server: Addr< App > ,
	form  : HtmlElement ,
}


impl ConnectForm
{
	pub fn new( server: Addr< App > ) -> Self
	{
		let cnick: HtmlInputElement = get_id( "connect_nick" ).unchecked_into();
		cnick.set_value( random_name() );

		// hide the connect form
		//
		let form : HtmlElement = get_id( "connect_form" ).unchecked_into();

		Self { server, form  }
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



pub struct ConnSubmitEvt { pub e: Event }

impl Message for ConnSubmitEvt { type Return = (); }

// Unfortunately thespis requires Send right now, and HtmlElement isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for ConnSubmitEvt {}


impl Handler< ConnSubmitEvt > for ConnectForm
{
	fn handle_local( &mut self, msg: ConnSubmitEvt ) -> ReturnNoSend<()>
	{
		info!( "Connect button clicked" );

		Box::pin( async move
		{
			msg.e.prevent_default();

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
			sm.register_handler::< ServerMsg >( self.server.clone_box() );

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
					self.form.style().set_property( "display", "none" ).expect_throw( "set cform display none" );

					let connection = Connected
					{
						peer_addr   ,
						server_addr ,
						welcome     ,
						ws          ,
					};

					self.server.call( connection ).await.expect_throw( "call" );
				}

				Err(e) =>
				{
					error!( "{}", e );

					let cerror: HtmlElement = get_id( "connect_error" ).unchecked_into();

					match e.kind()
					{
						ChatErrKind::NickInvalid   =>
						{
							cerror.set_inner_text( "The nick name is invalid. It must be between 1 and 15 word characters." );
							cerror.style().set_property( "display", "block" ).expect_throw( "set display block on cerror" );
						}

						ChatErrKind::NickInuse     =>
						{
							cerror.set_inner_text( "The nick name is already in use. Please choose another." );
							cerror.style().set_property( "display", "block" ).expect_throw( "set display block on cerror" );
						}

						ChatErrKind::AlreadyJoined => {}

						_ => unreachable!( "Unknown ChatErrKind" ),
					}
				}
			};
		})
	}

	fn handle( &mut self, _: ConnSubmitEvt ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}



pub struct ConnResetEvt { pub e: Event }

impl Message for ConnResetEvt { type Return = (); }

// Unfortunately thespis requires Send right now, and HtmlElement isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for ConnResetEvt {}


impl Handler< ConnResetEvt > for ConnectForm
{
	fn handle_local( &mut self, msg: ConnResetEvt ) -> ReturnNoSend<()> { Box::pin( async move
	{
		let _ = &msg; // need to capture the entire struct.
		msg.e.prevent_default();

		let cnick : HtmlInputElement = get_id( "connect_nick" ).unchecked_into();
		let curl  : HtmlInputElement = get_id( "connect_url"  ).unchecked_into();
		let cerror: HtmlElement      = get_id( "connect_error" ).unchecked_into();

		cnick.set_value( random_name()         );
		curl .set_value( "ws://127.0.0.1:3012" );

		cerror.style().set_property( "display", "none" ).expect_throw( "set display none on cerror" );
		cerror.set_inner_text( "" );
	})}

	fn handle( &mut self, _msg: ConnResetEvt ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on threadpool")
	}
}
