use
{
	crate::{ import::*, * },
	super::RequestError    ,
};


/// Type representing Messages coming in over the wire, for internal use only.
//
pub(super) struct Incoming
{
	pub(crate) msg: Result<WireFormat, ThesRemoteErr>
}

impl Message for Incoming
{
	type Return = ();
}


/// Handler for incoming messages.
///
/// Currently we use pharos to allow observers to follow events from this connection.
/// Pharos is async and can use bounded channels. We await pharos, which means that
/// it will give back pressure if it can't follow.
///
/// TODO: is this desired? If so support it officially and document it.
//
impl Handler<Incoming> for Peer
{
fn handle( &mut self, incoming: Incoming ) -> Return<'_, ()>
{

async move
{
	match &mut self.addr
	{
		Some(ref mut addr) => addr,

		// we no longer have our address, we're shutting down. we can't really do anything
		// without our address we won't have the sink for the connection either. We can
		// no longer send outgoing messages. Don't process any more incoming message.
		//
		None => return,
	};


	let frame = match incoming.msg
	{
		Ok ( mesg  ) => mesg,
		Err( error ) =>
		{
			// Can be:
			// - MessageSizeExceeded (Codec)
			// - DeserializeWireFormat (WireFormat)
			//
			self.handle( RequestError::from( error ) ).await;

			return
		}
	};


	// algorithm for incoming messages. Options are:
	//
	// 1. incoming send/call               for local/relayed/unknown actor     (6 options)
	// 2.       response to outgoing call from local/relayed actor             (2 options)
	// 3.       response to outgoing call from local/relayed actor (timed out) (2 options)
	// 4. error response to outgoing call from local/relayed actor             (2 options)
	//
	// 4 possibilities with ServiceID and ConnID. These can be augmented with
	// predicates about our local state (sid in local table, routing table, unknown), + the codec
	// which gives us largely the 10 needed states:
	//
	// SID  present -> always tells us if it's for local/relayed/unknown actor
	//                 based on our routing tables
	//
	//                 if it's null, it means the message is meant for this peer (ConnectionError).
	//
	// (leaves distinguishing between send/call/response/error)
	//
	// CID   absent  -> Send
	// CID   unknown -> Call or timed out response (SID full)
	//
	// CID   present -> Return/Error
	//
	// (leaves Return/Error)
	//
	// sid null      -> ConnectionError
	//
	// I think these should never fail, because they accept random data in the current implementation.
	// However, since it's implementation dependant, and we are generic, we can't know that. It's probably
	// safer to assume that if these do fail we close the connection because all bet's are off for following
	// messages.
	//
	let sid = frame.service();
	let cid = frame.conn_id();


	// It's a connection error from the remote peer
	//
	if sid.is_null()
	{
		self.remote_conn_err( frame.mesg(), cid ).await;
	}


	// it's an incoming send
	//
	else if cid.is_null()
	{
		self.incoming_send( sid, frame ).await;
	}


	// it's a succesful response to a (relayed) call
	//
	else if let Some( channel ) = self.responses.remove( &cid )
	{
		// It's a response
		//
		trace!( "{}: Incoming Return", self.identify() );

		// Normally if this fails it means the receiver of the channel was dropped...
		//
		if channel.send( Ok( frame ) ).is_err()
		{
			warn!( "{}: Received response for dead actor, cid: {}.", self.identify(), cid );
		}
	}

	// There is a CID, so it's a response, but it's not in our self.responses, so it has timed out.
	// We are no longer waiting for this response, so we can only drop it.
	//
	else if sid.is_full()
	{
		warn!( "{}: Received response for a timed out outgoing request, cid: {}. Dropping response.", self.identify(), cid );
	}


	// it's a call (!cid.is_null() and cid is unknown and there is a sid not null)
	//
	else
	{
		if let Some( ref bp ) = self.backpressure
		{
			bp.remove_slots( NonZeroUsize::new(1).unwrap() );
		}

		self.incoming_call( cid, sid, frame ).await;
	}

}.boxed() // End of async move

} // end of handle
} // end of impl Handler


impl Peer
{
	// It's a connection error from the remote peer
	//
	// This includes failing to deserialize our messages, failing to relay, unknown service, ...
	//
	async fn remote_conn_err( &mut self, msg: Bytes, cid: ConnID )
	{
		// We can correctly interprete the error
		//
		if let Ok( err ) = serde_cbor::from_slice::<ConnectionError>( &msg )
		{
			// We need to report the connection error to the caller
			//
			if let Some( channel ) = self.responses.remove( &cid )
			{
				// If this returns an error, it means the receiver was dropped, so if they no longer
				// care for the result, neither do we, so ignoring the result.
				//
				let _ = channel.send( Err( err ) );

				// Since this was not our error, just relay the response.
				//
				return
			}


			error!( "{}: Remote error: {:?}", self.identify(), &err );

			// Notify observers
			//
			let shine = PeerEvent::RemoteError( err.clone() );
			self.pharos.send( shine ).await.expect( "pharos not closed" );

		}

		// Since we can't deserialize it, we can't do much except log.
		//
		else
		{
			error!( "{}: We received an error message from a remote peer, \
				      but couldn't deserialize it", self.identify() )
			;
		}
	}



	// Process incoming Send requests.
	//
	async fn incoming_send
	(
		&mut self            ,
		sid     : ServiceID  ,
		frame   : WireFormat ,
	)
	{
		if self.closed { return };

		let identity = self.identify();

		trace!( "{}: Incoming Send, sid: {}", identity, &sid );

		let self_addr = self.addr.as_mut().take().unwrap();


		if let Some( sm ) = self.services.get( &sid )
		{
			trace!( "{}: Incoming Send", &identity );

			// Send to handling actor,
			//
			let fut = match sm.send_service( frame )
			{
				Ok(f) => f,

				Err(e) =>
				{
					let ctx = Peer::err_ctx( &self_addr, sid, None, "sm.send_service".to_string() );

					let err = match e
					{
						ThesRemoteErr::NoHandler  {..} => ThesRemoteErr::NoHandler  {ctx},
						ThesRemoteErr::Downcast   {..} => ThesRemoteErr::Downcast   {ctx},
						ThesRemoteErr::Deserialize{..} => ThesRemoteErr::Deserialize{ctx},
						_                              => unreachable!(),
					};

					// If we are no longer around, just log the error.
					//
					if self_addr.send( RequestError::from( err.clone() ) ).await.is_err()
					{
						error!( "{}: {}.", identity, &err );
					}

					return
				}
			};


			if self.nursery.as_ref().unwrap().nurse( fut ).is_err()
			{
				let err = ThesRemoteErr::Spawn
				{
					ctx: Peer::err_ctx( &self_addr, sid, None, "sm.send_service".to_string() )
				};


				// If we are no longer around, just log the error.
				//
				if self_addr.send( RequestError::from( err.clone() ) ).await.is_err()
				{
					error!( "{}: {}.", identity, &err );
				}
			}
		}


		// service_id unknown => send back and log error
		//
		else
		{
			let ctx = Peer::err_ctx( &self_addr, sid, None, "Process incoming Send".to_string() );

			self.handle(
			{
				RequestError::from( ThesRemoteErr::UnknownService{ ctx } )

			}).await;
		}
	}



	async fn incoming_call
	(
		&mut self            ,
		cid     : ConnID     ,
		sid     : ServiceID  ,
		frame   : WireFormat ,
	)
	{
		if self.closed { return }

		trace!( "{}: Incoming Call", self.identify() );

		let mut self_addr = self.addr.as_ref().unwrap().clone();
		let identity = self.identify();

		// It's a call for a local actor
		//
		if let Some( sm ) = self.services.get( &sid )
		{
			trace!( "{}: Incoming Call", &identity );

			let addr = self_addr.clone();
			let fut = match sm.call_service( frame, addr )
			{
				Ok(f) => f,

				Err(e) =>
				{
					let ctx = Peer::err_ctx( &self_addr, sid, None, "sm.call_service".to_string() );

					let err = match e
					{
						ThesRemoteErr::NoHandler  {..} => ThesRemoteErr::NoHandler  {ctx},
						ThesRemoteErr::Downcast   {..} => ThesRemoteErr::Downcast   {ctx},
						ThesRemoteErr::Deserialize{..} => ThesRemoteErr::Deserialize{ctx},
						_                              => unreachable!(),
					};

					// If we are no longer around, just log the error.
					//
					if self_addr.send( RequestError::from( err.clone() ) ).await.is_err()
					{
						error!( "{}: {}.", identity, &err );
					}

					return
				}
			};

			// Call handling actor,
			//
			if self.nursery.as_ref().unwrap().nurse( fut ).is_err()
			{
				let err = ThesRemoteErr::Spawn
				{
					ctx: Peer::err_ctx( &self_addr, sid, None, "sm.call_service".to_string() )
				};


				// If we are no longer around, just log the error.
				//
				if self_addr.send( RequestError::from( err.clone() ) ).await.is_err()
				{
					error!( "{}: {}.", &identity, &err );
				}
			}
		}


		// service_id unknown => send back and log error
		//
		else
		{
			let ctx = Self::err_ctx( &self_addr, sid, cid, "Process incoming Call".to_string() );

			self.handle(
			{
				RequestError::from( ThesRemoteErr::UnknownService{ ctx } )

			}).await;
		}
	}
}
