use crate :: { import::*, * };

#[component]
pub fn ChatFormDom( cx: Scope, addr: Addr<ChatForm> ) -> impl IntoView
{
	let addr1 = addr.clone();
	let addr2 = addr.clone();

	let on_submit = move |e| chat_submit(e, addr1.clone());
	let on_reset  = move |e| chat_reset (e, addr2.clone());
	let on_enter  = move |e| chat_enter (e, addr.clone());

	view! { cx,

		// <!-- if we don't put the javascript link, it will still submit when programatorically triggering
		// click on the button... -->

		<form id="chat_form" action="javascript:void(0);" on:submit = on_submit on:reset = on_reset >
			<textarea id="chat_input" on:keypress = on_enter ></textarea>

			<input id="chat_submit"     type = "submit" value = "Send"       />
			<input id="chat_disconnect" type = "reset"  value = "Disconnect" />
		</form>
	}
}

fn chat_submit(e: web_sys::SubmitEvent, mut chat_form: Addr<ChatForm> )
{
	Bindgen.spawn_local( async move
	{
		chat_form.send( ChatSubmitEvt{e: e.into()} ).await
			.expect_throw( "send ChatSubmitEvt")
		;

	}).expect_throw( "spawn send ChatSubmitEvt");
}


fn chat_reset(e: Event, mut chat_form: Addr<ChatForm> )
{
	Bindgen.spawn_local( async move
	{
		chat_form.send( ChatResetEvt{e} ).await.expect_throw( "send ChatResetEvt");

	}).expect_throw( "spawn send ChatResetEvt");
}


fn chat_enter(e: web_sys::KeyboardEvent, mut chat_form: Addr<ChatForm> )
{
	Bindgen.spawn_local( async move
	{
		// We also trigger this if the user types Enter.
		// Shift+Enter let's the user create a new line in the message.
		//
		if e.has_type::<KeyboardEvent>()
		{
			let evt: KeyboardEvent = e.clone().unchecked_into();

			if  evt.code() != "Enter"  ||  evt.shift_key()
			{
				return;
			}
		}

		chat_form.send(ChatSubmitEvt{e: e.into()}).await.expect_throw( "send ChatSubmitEvt" );

	}).expect_throw( "spawn send send ChatSubmitEvt keypress" );
}

#[ derive( Actor ) ]
//
pub struct ChatForm
{
	app        : Addr< App        > ,
	chat_window: Addr< ChatWindow > ,
}


impl ChatForm
{
	pub fn new( app: Addr<App>, chat_window: Addr< ChatWindow >, self_addr: Addr< Self > ) -> Self
	{
		mount_to_global( get_id( "page_chat" ), Operation::Append, move |cx|
		{
			view! { cx, <ChatFormDom addr=self_addr /> }
		});

		Self { app, chat_window }
	}
}



pub struct ChatSubmitEvt { pub e: Event }

impl Message for ChatSubmitEvt { type Return = (); }

// Unfortunately thespis requires Send right now, and HtmlElement isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for ChatSubmitEvt {}


impl Handler< ChatSubmitEvt > for ChatForm
{
	#[async_fn_local] fn handle_local( &mut self, msg: ChatSubmitEvt )
	{
		info!( "Chat submit button clicked" );

		msg.e.prevent_default();

		let nickre   = Regex::new( r"^/nick (\w{1,15})" ).unwrap();

		// Note that we always add a newline below, so we have to match it.
		//
		let helpre   = Regex::new(r"^/help\n$").unwrap();

		let textarea = get_id( "chat_input" );
		let textarea: &HtmlTextAreaElement = textarea.unchecked_ref();

		debug!( "on_submit" );


		let text = textarea.value().trim().to_string() + "\n";
		textarea.set_value( "" );
		let _ = textarea.focus();

		if text == "\n" { return; }


		// if this is a /nick somename message
		//
		if let Some( cap ) = nickre.captures( &text )
		{
			debug!( "handle set nick: {:#?}", &text );

			self.app.send( SetNick{ nick: cap[1].to_string() } ).await.expect_throw( "chang nick" );
		}


		// if this is a /help message
		//
		else if helpre.is_match( &text )
		{
			debug!( "handle /help: {:#?}", &text );

			self.chat_window.send( PrintHelp{} ).await.expect_throw( "chang nick" );

			return;
		}


		else
		{
			debug!( "handle send: {:#?}", &text );

			self.app.send( NewChatMsg(text) ).await.expect_throw( "chang nick" );
		}
	}

	fn handle( &mut self, _: ChatSubmitEvt ) -> Return<()>
	{
		unreachable!( "Cannot be spawned on a threadpool" );
	}
}


// Disconnect button
//
pub struct ChatResetEvt { pub e: Event }

impl Message for ChatResetEvt { type Return = (); }

// Unfortunately thespis requires Send right now, and Event isn't Send.
// When WASM gains threads we need to fix this.
//
unsafe impl Send for ChatResetEvt {}


impl Handler< ChatResetEvt > for ChatForm
{
	#[async_fn_nosend] fn handle_local( &mut self, msg: ChatResetEvt )
	{
		msg.e.prevent_default();

		self.app.send( Disconnect {} ).await.expect_throw( "send disconnect" );
	}
}


