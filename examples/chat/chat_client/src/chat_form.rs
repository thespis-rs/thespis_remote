use crate :: { import::*, * };

#[ derive( Actor ) ]
//
pub struct ChatForm
{
	app        : Addr< App        > ,
	chat_window: Addr< ChatWindow > ,
}


impl ChatForm
{
	pub fn new( app: Addr<App>, chat_window: Addr< ChatWindow > ) -> Self
	{
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

	fn handle_local( &mut self, msg: ChatSubmitEvt ) -> ReturnNoSend<()>
	{
		info!( "Chat submit button clicked" );

		Box::pin( async move
		{
			// We also trigger this if the user types Enter.
			// Shift+Enter let's the user create a new line in the message.
			//
			if msg.e.has_type::<KeyboardEvent>()
			{
				let evt: KeyboardEvent = msg.e.clone().unchecked_into();

				if  evt.code() != "Enter"  ||  evt.shift_key()
				{
					return;
				}
			}


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

				return;
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

		})
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
	fn handle( &mut self, msg: ChatResetEvt ) -> Return<()> { Box::pin( async move
	{
		msg.e.prevent_default();

		self.app.send( Disconnect {} ).await.expect_throw( "send disconnect" );
	})}
}


