use
{
	crate::{ import::*, *, WireType },
	super::RequestError              ,
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
	#[async_fn] fn handle( &mut self, incoming: Incoming )
	{
		// We're shut down. we can't really do anything useful.
		//
		if self.closed { return }

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


		let sid = frame.service();
		let cid = frame.conn_id();


		// TODO: when we have benchmarks, verify if it's better to return boxed submethods here
		// rather than awaiting. Implies the rest of this method can run sync.
		//
		match frame.kind()
		{
			WireType::ConnectionError => self.remote_conn_err( frame.mesg(), cid ).await,
			WireType::IncomingSend    => self.incoming_send  ( sid, frame        ).await,
			WireType::IncomingCall    => self.incoming_call  ( cid, sid, frame   ).await,

			WireType::CallResponse =>
			{
				// it's a succesful response to a (relayed) call
				//
				if let Some( channel ) = self.responses.remove( &cid )
				{
					// It's a response
					//
					trace!( "{}: Incoming Return", self.identify() );

					// Normally if this fails it means the receiver of the channel was dropped...
					//
					if channel.send( Ok(frame) ).is_err()
					{
						warn!( "{}: Received response for dead actor, cid: {}.", self.identify(), cid );
					}
				}

				// There is a CID, so it's a response, but it's not in our self.responses, so it has timed out.
				// We are no longer waiting for this response, so we can only drop it.
				//
				else
				{
					warn!( "{}: Received response for a timed out outgoing request, cid: {}. Dropping response.", self.identify(), cid );
				}
			}
		}
	}
}


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
			let shine = PeerEvent::RemoteError( err );
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
		let identity = self.identify();

		trace!( "{}: Incoming Send, sid: {}", &identity, &sid );

		let ctx = self.ctx( sid.clone(), None, "Peer: Handle incoming send" );

		let sm = match self.services.get( &sid )
		{
			Some( sm ) => sm,

			// service_id unknown => send back and log error
			//
			None =>
			{
				let err = ThesRemoteErr::UnknownService{ ctx };

				self.handle( RequestError::from( err ) ).await;

				return;
			}
		};


		// Send to handling actor,
		//
		let fut = match sm.send_service( frame, ctx )
		{
			Ok(f) => f,

			Err(e) =>
			{
				let ctx = self.ctx( sid.clone(), None, "sm.send_service" );

				let err = match e
				{
					ThesRemoteErr::NoHandler  {..} => ThesRemoteErr::NoHandler  {ctx},
					ThesRemoteErr::Downcast   {..} => ThesRemoteErr::Downcast   {ctx},
					ThesRemoteErr::Deserialize{..} => ThesRemoteErr::Deserialize{ctx},
					_                              => unreachable!(),
				};


				// If we are no longer around, just log the error.
				//
				return self.handle( RequestError::from(err) ).await;
			}
		};


		if self.nursery.nurse( fut ).is_err()
		{
			let ctx = self.ctx( sid.clone(), None, "sm.send_service" );

			let err = ThesRemoteErr::Spawn { ctx };

			// If we are no longer around, just log the error.
			//
			self.handle( RequestError::from(err) ).await
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

		if let Some( ref bp ) = self.backpressure
		{
			bp.remove_slots( NonZeroUsize::new(1).unwrap() );
		}

		trace!( "{}: Incoming Call, sid: {}, cid: {}", self.identify(), &sid, &cid );

		let ctx = self.ctx( sid.clone(), cid.clone(), "Peer: Handle incoming call" );


		// Find our handler.
		//
		let sm = match self.services.get( &sid )
		{
			Some( sm ) => sm,

			// service_id unknown => send back and log error
			//
			None =>
			{
				let err = ThesRemoteErr::UnknownService{ ctx };

				return self.handle( RequestError::from( err ) ).await;
			}
		};


		// Get future from service map.
		//
		let fut = match sm.call_service( frame, ctx.clone() )
		{
			Ok (f) => f,
			Err(e) => return self.handle( RequestError::from( e ) ).await,
		};


		// Call handling actor,
		//
		if self.nursery.nurse( fut ).is_err()
		{
			let err = ThesRemoteErr::Spawn{ ctx	};

			self.handle( RequestError::from( err ) ).await;
		}
	}
}
