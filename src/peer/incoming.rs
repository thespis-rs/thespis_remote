use
{
	crate::{ import::*, *, WireType },
	super::RequestError              ,
};


/// Type representing Messages coming in over the wire, for internal use only.
//
pub(super) struct Incoming<Wf>
{
	pub(crate) msg: Result<Wf, WireErr>
}

impl<Wf: Send + 'static> Message for Incoming<Wf>
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
impl<Wf: WireFormat + Send + 'static> Handler<Incoming<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, incoming: Incoming<Wf> )
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
				// - WireErr::MessageSizeExceeded (Codec)
				// - WireErr::Deserialize (BytesFormat)
				//
				let err = PeerErr::WireFormat{ source: error, ctx: self.ctx( None, None, "Incoming message" ) };

				self.handle( RequestError::from( err ) ).await;

				return
			}
		};


		let sid  = frame.service();
		let cid  = frame.conn_id();
		let kind = frame.kind();
		let msg  = frame.mesg();

		// TODO: when we have benchmarks, verify if it's better to return boxed submethods here
		// rather than awaiting. Implies the rest of this method can run sync.
		//
		match kind
		{
			WireType::ConnectionError => self.remote_conn_err( msg, cid        ).await,
			WireType::IncomingSend    => self.incoming_send  ( sid, frame      ).await,
			WireType::IncomingCall    => self.incoming_call  ( cid, sid, frame ).await,

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


impl<Wf: WireFormat> Peer<Wf>
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

			// Notify observers
			//
			let shine = PeerEvent::RemoteError( err );

			// If pharos is closed, we already panicked... so except is fine.
			//
			self.pharos.send( shine ).await.expect( "pharos not closed" );

		}

		// Since we can't deserialize it, we can't do much except log.
		//
		else
		{
			let ctx = self.ctx( None, cid, "We received an error message from a remote peer, but couldn't deserialize it" );
			let err = PeerErr::Deserialize{ ctx };
			let shine = PeerEvent::Error(err);

			// If pharos is closed, we already panicked... so except is fine.
			//
			self.pharos.send( shine ).await.expect( "pharos not closed" );
		}
	}



	// Process incoming Send requests.
	//
	async fn incoming_send
	(
		&mut self           ,
		sid     : ServiceID ,
		frame   : Wf        ,
	)
	{
		let identity = self.identify();

		trace!( "{}: Incoming Send, sid: {}", &identity, &sid );

		let ctx = self.ctx( sid, None, "Peer: Handle incoming send" );

		let sm = match self.services.get( &sid )
		{
			Some( sm ) => sm,

			// service_id unknown => send back and log error
			//
			None =>
			{
				let err = PeerErr::UnknownService{ ctx };

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
				let ctx = self.ctx( sid, None, "sm.send_service" );

				let err = match e
				{
					PeerErr::NoHandler  {..} => PeerErr::NoHandler  {ctx},
					PeerErr::Deserialize{..} => PeerErr::Deserialize{ctx},
					_                              => unreachable!(),
				};


				// If we are no longer around, just log the error.
				//
				return self.handle( RequestError::from(err) ).await;
			}
		};


		if self.nursery.nurse( fut ).is_err()
		{
			let ctx = self.ctx( sid, None, "sm.send_service" );

			let err = PeerErr::Spawn { ctx };

			// If we are no longer around, just log the error.
			//
			self.handle( RequestError::from(err) ).await
		}
	}



	async fn incoming_call
	(
		&mut self           ,
		cid     : ConnID    ,
		sid     : ServiceID ,
		frame   : Wf        ,
	)
	{
		if self.closed { return }

		if let Some( ref bp ) = self.backpressure
		{
			bp.remove_slots( NonZeroUsize::new(1).unwrap() );
		}

		trace!( "{}: Incoming Call, sid: {}, cid: {}", self.identify(), sid, cid );

		let ctx = self.ctx( sid, cid, "Peer: Handle incoming call" );


		// Find our handler.
		//
		let sm = match self.services.get( &sid )
		{
			Some( sm ) => sm,

			// service_id unknown => send back and log error
			//
			None =>
			{
				let err = PeerErr::UnknownService{ ctx };

				return self.handle( RequestError::from( err ) ).await;
			}
		};


		// Get future from service map.
		//
		let fut = match sm.call_service( frame, ctx.clone() )
		{
			Ok (f) => f,
			Err(e) => return self.handle( RequestError::from(e) ).await,
		};


		// Call handling actor,
		//
		if self.nursery.nurse( fut ).is_err()
		{
			let err = PeerErr::Spawn{ ctx	};

			self.handle( RequestError::from( err ) ).await;
		}
	}
}
