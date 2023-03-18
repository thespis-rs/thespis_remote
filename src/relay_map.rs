use crate :: { import::*, *, peer::Response };



/// Register services to be relayed to backend providers. The difference with the `service_map` macro, which is
/// used for local handlers is that handlers here don't have to implement `Handler<M>` for the actual message type.
/// They only have to implement `Handler<BytesFormat>` (for sends) and `Handler<peer::Call>` for calls.
///
/// # Example
/// Please check out the [`tracing` example](https://github.com/thespis-rs/thespis_remote/blob/dev/examples/tracing.rs) which uses relays.
//
pub struct RelayMap<Wf>
{
	// Unfortunately we need the Mutex because mpsc::Sender's are generally not `Sync` and `ServiceMap`
	// definitely has to be `Sync`. We cannot have a guarantee that cloning a channel sender is thread safe.
	// We also cannot use `RwLock` because that only protects mut access, but cloning only uses immutable access.
	//
	handler : Mutex<ServiceHandler<Wf>> ,
	services: Vec<ServiceID>            ,
}


impl<Wf> RelayMap<Wf>
{
	/// Create a RelayMap.
	//
	pub fn new( handler: ServiceHandler<Wf>, services: Vec<ServiceID> ) -> Self
	{
		Self { handler: Mutex::new( handler ), services }
	}
}



impl<Wf: WireFormat> ServiceMap<Wf> for RelayMap<Wf>
{
	/// Send a message to a handler. This should take care of deserialization.
	//
	fn send_service( &self, msg: Wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	{
		trace!( "RelayMap: Incoming Send for relayed actor." );

		let sid = msg.sid();

		// This sid should be in our map.
		//
		match &*self.handler.lock()
		{
			ServiceHandler::Address( a ) =>
			{
				let mut a = a.clone_box();

				let task = async move
				{
					match a.send( msg ).await
					{
						Ok (_) => Ok ( Response::Nothing           ) ,
						Err(_) => Err( PeerErr::HandlerDead{ ctx } ) ,
					}
				};

				Ok( task.boxed() )
			}


			ServiceHandler::Closure( c ) =>
			{
				let mut a = c(&sid);

				let task = async move
				{
					match a.send( msg ).await
					{
						Ok (_) => Ok ( Response::Nothing           ) ,
						Err(_) => Err( PeerErr::HandlerDead{ ctx } ) ,
					}
				};

				Ok( task.boxed() )
			}
		}
	}


	/// Call a Service.
	/// This should take care of deserialization. The return address is the address of the peer
	/// to which the serialized answer shall be send.
	//
	fn call_service( &self, frame: Wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	{
		trace!( "RelayMap: Incoming Call for relayed actor." );

		let sid = frame.sid();

		match &*self.handler.lock()
		{
			ServiceHandler::Address( a ) => Ok( make_call( a.clone_box(), frame, ctx ).boxed() ),
			ServiceHandler::Closure( c ) => Ok( make_call( c(&sid)      , frame, ctx ).boxed() ),
		}
	}


	fn services( &self ) -> Box<dyn Iterator<Item = &ServiceID> + '_ >
	{
		Box::new( self.services.iter() )
	}


	fn apply_backpressure( &self ) -> bool
	{
		false
	}
}


#[ allow(clippy::needless_return) ]
//
async fn make_call<T, Wf: WireFormat + Send + 'static>( mut relay: Box<T>, frame: Wf, ctx: PeerErrCtx )

	-> Result<Response<Wf>, PeerErr >

	where T: Address<Call<Wf>, Error=ThesErr> + ?Sized

{
	let cid        = frame.cid();
	let ctx        = ctx.context( "Process incoming Call to relay".to_string() );
	let peer_id    = ctx.peer_id;
	let relay_id   = relay.id();
	let relay_name = relay.name();
	let relay_gone = PeerErr::RelayGone{ ctx, relay_id, relay_name };
	let new_call   = Call::new( frame );

	// Peer for relay still online.
	// FIXME: use map_err when rustc supports it... currently relay_gone would have to be cloned.
	//
	let called = match relay.call( new_call ).await
	{
		Ok(x) => x,

		// For now can only be mailbox closed.
		//
		Err(_) => return Err( relay_gone ),
	};


	// Sending out over the sink (connection) didn't fail
	//
	let receiver = match called
	{
		Ok(x) => x,

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
		Ok( mut resp ) =>
		{
			trace!( "Peer {:?}: Got response from relayed call, sending out.", &peer_id );

			// Put the old cid back!
			//
			resp.set_cid( cid );

			Ok( Response::CallResponse(CallResponse::new(resp)) )
		},

		// The relayed remote had errors while processing the request, such as deserialization.
		// Our own peer might send a Timeout error back as well.
		//
		Err(e) =>
		{
			let wire_format = Peer::prep_error( cid, &e );

			Ok( Response::WireFormat(wire_format) )
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
impl<Wf> fmt::Debug for RelayMap<Wf>
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
