use crate :: { import::*, * };



/// Register services to be relayed to other backend providers. The difference with the `service_map` macro, which is
/// used for local handlers is that handlers here don't have to implement `Handler<M>` for the actual message type.
/// They only have to implement `Handler<WireFormat>` (for sends) and `Handler<peer::Call>` for calls.
//
pub struct RelayMap
{
	// I decided not to take a static reference to ServiceID, because it seems kind or limiting how people
	// can store them. In this case, the process does not need to compile in the actual handlers.
	// ServiceID is just 16 bytes of data.
	//
	handler : Mutex<ServiceHandler> ,
	services: Vec<ServiceID>        ,
}


impl RelayMap
{
	/// Create a RelayMap.
	//
	pub fn new( handler: ServiceHandler, services: Vec<ServiceID> ) -> Self
	{
		Self { handler: Mutex::new( handler ), services }
	}
}



impl ServiceMap for RelayMap
{
	/// Send a message to a handler. This should take care of deserialization.
	//
	fn send_service( &self, msg: WireFormat, ctx: ErrorContext )

		-> Result< Pin<Box< dyn Future< Output=Result<(), ThesRemoteErr> > + Send >>, ThesRemoteErr >
	{
		trace!( "RelayMap: Incoming Send for relayed actor." );

		let sid = msg.service();

		// This sid should be in our map.
		//
		match &*self.handler.lock()
		{
			ServiceHandler::Address( a ) =>
			{
				let mut a = a.clone_box();

				let task = async move
				{
					a.send( msg ).await.map_err( |_|
					{
						ThesRemoteErr::HandlerDead{ ctx }
					})
				};

				Ok( task.boxed() )
			}


			ServiceHandler::Closure( c ) =>
			{
				let mut a = c(&sid);

				let task = async move
				{
					a.send( msg ).await.map_err( |_|
					{
						ThesRemoteErr::HandlerDead{ ctx }
					})
				};

				Ok( task.boxed() )			}
		}
	}


	/// Call a Service.
	/// This should take care of deserialization. The return address is the address of the peer
	/// to which the serialized answer shall be send.
	//
	fn call_service( &self, frame: WireFormat, peer: Addr<Peer>, ctx: ErrorContext )

		-> Result< Pin<Box< dyn Future< Output=Result<(), ThesRemoteErr> > + Send >>, ThesRemoteErr >
	{
		trace!( "RelayMap: Incoming Call for relayed actor." );

		let sid = frame.service();

		match &*self.handler.lock()
		{
			ServiceHandler::Address( a ) => Ok( make_call( a.clone_box(), frame, peer, ctx ).boxed() ),
			ServiceHandler::Closure( c ) => Ok( make_call( c(&sid)      , frame, peer, ctx ).boxed() ),
		}
	}

	// We need to make a Vec here because the hashmap.keys() doesn't have a static lifetime.
	//
	fn services( &self ) -> Vec<ServiceID>
	{
		self.services.clone()
	}
}


#[ allow(clippy::needless_return) ]
//
async fn make_call<T>( mut relay: Box<T>, frame: WireFormat, mut peer: Addr<Peer>, ctx: ErrorContext )

	-> Result<(), ThesRemoteErr >

	where T: Address<Call, Error=ThesErr> + ?Sized

{
	let cid        = frame.conn_id();
	let ctx        = ctx.context( "Process incoming Call to relay".to_string() );

	let relay_id   = relay.id();
	let relay_name = relay.name();
	let relay_gone = ThesRemoteErr::RelayGone{ ctx, relay_id, relay_name };

	// Peer for relay still online.
	// TODO: use map_err when rustc supports it... currently relay_gone would have to be cloned.
	//
	let called = match relay.call( Call::new( frame ) ).await
	{
		Ok(c) => c,

		// For now can only be mailbox closed.
		//
		Err(_) => return Err( relay_gone ),
	};


	// Sending out over the sink (connection) didn't fail
	//
	let receiver = match called
	{
		Ok(c) => c,

		// Sending out call to relayed failed. This normally only happens if the connection
		// was closed, or a network transport malfunctioned.
		//
		Err(_) => return Err( relay_gone ),
	};


	// The channel was not dropped before resolving, so the relayed connection didn't close
	// until we got a response.
	//
	let received = receiver.await

		// This can only happen if the sender got dropped. Eg, if the remote relay goes down
		// Inform peer that their call failed because we lost connection to the relay after
		// it was sent out.
		//
		.map_err( |_| relay_gone )?
	;



	match received
	{
		// The remote managed to process the message and send a result back
		// (no connection errors like deserialization failures etc)
		//
		Ok( resp ) =>
		{
			trace!( "Peer {}: Got response from relayed call, sending out.", &peer.id() );

			// This can fail if we are no longer connected, in which case there isn't much to do.
			//
			if peer.send( resp ).await.is_err()
			{
				error!( "Peer {}: processing incoming call for relay: peer to client is closed before we finished sending a response to the request.", &peer.id() );
			}

			Ok(())
		},

		// The relayed remote had errors while processing the request, such as deserialization.
		// Our own peer might send a Timeout error back as well.
		//
		Err(e) =>
		{
			let wire_format = Peer::prep_error( cid, &e );

			// Just forward the error to the client.
			//
			if peer.send( wire_format ).await.is_err()
			{
				error!( "Peer {}: processing incoming call for relay failed and peer to client is closed before we finished sending an error back.", &peer.id() );
			}

			Ok(())
		},
	}
}



/// Will print something like:
///
/// ```ignore
/// "RelayMap, handler: ServiceHandler: Address: id: {}, name: <name of handler>, services:
/// {
///    sid: 0xbcc09d3812378e171ad366d75f687757
///    sid: 0xbcc09d3812378e17e1a1e89b512c025a
/// }"
/// ```
//
impl fmt::Debug for RelayMap
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "RelayMap, handler: {}, services:\n{{\n", &*self.handler.lock() )?;

		for sid in &self.services
		{
			writeln!( f, "\tsid: 0x{:02x}", sid )?;
		}

		write!( f, "}}" )
	}
}
