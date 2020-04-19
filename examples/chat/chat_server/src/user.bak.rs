use crate::{ import::*, Connection, Clients };

static CLIENT: AtomicUsize = AtomicUsize::new(0);




pub struct User
{
	pub nick: Arc<RwLock<String>>,
	pub sid : usize              ,
	pub conn: Connection         ,
}



impl User
{
	pub async fn from_connection( conn: Connection, clients: &Clients ) -> Result< Self, Box< dyn ErrorTrait > >
	{
		let first = conn.msgs.next().await.transpose()?;

		let nick = match first
		{
			Some( ClientMsg::Join( nick ) ) => nick,

			Some(_) => return Err( "need join message".into() ),
			_       => return Err( "connection closed".into() ),
		};

		// A unique sender id for this client
		//
		let sid: usize = CLIENT.fetch_add( 1, Ordering::Relaxed );

		match clients.validate_nick( sid, None, &nick )
		{
			Ok(_)    => {}
			Err(msg) =>
			{
				conn.out.send( msg ).await.expect( "send out msg" );
				return Err( "nick validation failed".into() );
			}
		};



		Ok( Self
		{
			sid,
			nick: Arc::new( RwLock::new( nick ) ),
			conn: conn,
		})
	}




	// Send a server message to all connected clients
	//
	pub async fn send( &self, msg: ServerMsg )
	{
		self.conn.out.send( msg.clone() ).await.expect( "send out msg" );
	}



	pub async fn incoming()
	{
		// Incoming messages. Ends when stream returns None or an error.
		//
		while let Some( msg ) = msgs.next().await
		{
			debug!( "received client message" );

			// TODO: handle io errors
			//
			let msg = match msg
			{
				Ok( msg ) => msg,
				_         => continue,
			};

			let time = Utc::now().timestamp();


			match msg
			{
				ClientMsg::Join( new_nick ) =>
				{
					// TODO: error
				}


				ClientMsg::SetNick( new_nick ) =>
				{
					debug!( "received SetNick" );


					let res = validate_nick( sid, &nick.read(), &new_nick );

					debug!( "validated new Nick" );

					match res
					{
						Ok ( m ) =>
						{
							broadcast( &m );
							*nick.write() = new_nick;
						}

						Err( m ) => send( sid, m ),
					}
				}


				ClientMsg::ChatMsg( txt ) =>
				{
					debug!( "received ChatMsg" );

					broadcast( &ServerMsg::ChatMsg { time, nick: nick.future_read().await.clone(), sid, txt } );
				}
			}
		};

		debug!( "Broke from msgs loop" );
	}
}
