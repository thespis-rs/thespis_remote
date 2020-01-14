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
			// - Deserialize (WireFormat)
			//
			self.handle( RequestError::from( error ) ).await;

			return
		}
	};


	// algorithm for incoming messages. Options are:
	//
	// 1. incoming send/call               for local/relayed/unknown actor (6 options)
	// 2.       response to outgoing call from local/relayed actor         (2 options)
	// 3. error response to outgoing call from local/relayed actor         (2 options)
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
	// CID   unknown -> Call
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
	// TODO: This next block is what requires that MS is Sync. I don't understand why. It says there is a
	// requirement of Send for &MS. Probably it's the call frame.service() which takes &self.
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
			warn!( "{}: Received response for dead actor, sid: {}, cid: {}.", self.identify(), sid, cid );
		}
	}


	// it's a call (!cid.is_null() and cid is unknown)
	//
	else
	{
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
	// TODO: when we sent a Call, it will have the cid in frame, so we should correctly react
	// to that and forward it to the original caller.
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
			// TODO: don't await pharos? it can block processing of incoming messages.
			// On the other hand, if pharos can't follow maybe we should consider it backpressure.
			//
			let shine = PeerEvent::RemoteError( err.clone() );
			self.pharos.send( shine ).await.expect( "pharos not closed" );

		}

		// TODO
		//
		else
		{
			error!( "{}: We received an error message from a remote peer, \
				      but couldn't deserialize it", self.identify() )
			;

			unimplemented!();
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

		trace!( "{}: Incoming Send, sid: {}", identity, &sid );

		let self_addr = self.addr.as_mut().take().expect( "Peer not closing down" );


		if let Some( typeid ) = self.services.get( &sid )
		{
			trace!( "{}: Incoming Send for local Actor", &identity );

			// unwrap: We are keeping our internal state consistent, if it's in
			// self.services, it's in self.service_maps.
			//
			let sm = &mut self.service_maps.get( &typeid ).unwrap();

			// Call handling actor,
			//
			if self.exec.spawn( sm.send_service( frame, self_addr.clone() ) ).is_err()
			{
				let err = ThesRemoteErr::Spawn
				{
					ctx: Peer::err_ctx( &self_addr, sid, None, "sm.call_service".to_string() )
				};


				// If we are no longer around, just log the error.
				//
				if self_addr.send( RequestError::from( err.clone() ) ).await.is_err()
				{
					error!( "{}: {}.", identity, &err );
				}
			}
		}


		// service_id in self.relay => Send to recipient found in self.relay.
		//
		else if let Some( relay_id ) = self.relayed.get( &sid )
		{
			trace!( "{}: Incoming Send for relayed Actor", identity );

			// unwrap: We are keeping our internal state consistent, if it's in
			// self.relayed, it's in peer.relays.
			//
			let relay = &mut self.relays.get_mut( &relay_id ).unwrap().0;

			// if this fails, well, the peer is no longer there. Warn remote and observers.
			// We let remote know we are no longer relaying to this service.
			// TODO: should we remove it from peer.relayed? Normally there is detection mechanisms
			//       already that should take care of this, but then if they work, we shouldn't be here...
			//       also, if we no longer use unbounded channels, this might block the task if their
			//       inbox if full, which will keep us from processing other incoming messages.
			//
			//
			let ctx = Peer::err_ctx( &self_addr, sid, None, "Process incoming Send to relay".to_string() );

			//////////////////////////////////////////////////////////////////////
			// TODO: this can block, in call we spawn a task to call the relay. //
			//////////////////////////////////////////////////////////////////////

			let res = relay.send( frame ).await;

			if res.is_err()
			{
				let err = RequestError::from( ThesRemoteErr::RelayGone
				{
					ctx                     ,
					relay_id  : relay.id()  ,
					relay_name: relay.name(),
				});

				self.handle( err ).await;
			};
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
		trace!( "{}: Incoming Call", self.identify() );

		let mut self_addr = self.addr.as_ref().expect( "Peer not closing down" ).clone();
		let identity = self.identify();

		// It's a call for a local actor
		//
		if let Some( typeid ) = self.services.get( &sid )
		{
			trace!( "{}: Incoming Call for local Actor", &identity );

			// We are keeping our internal state consistent, so the unwrap is fine. if it's in
			// self.services, it's in self.service_maps.
			//
			let sm = &mut self.service_maps.get( &typeid ).unwrap();

			// Call handling actor,
			//
			if self.exec.spawn( sm.call_service( frame, self_addr.clone() ) ).is_err()
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



		// Call for relayed actor
		//
		// service_id in peer.relay   => Create Call and send to recipient found in peer.relay.
		//
		// What could possibly go wrong???
		// - relay peer has been shut down (eg. remote closed connection)
		// - we manage to call, but then when we await the response, the relay goes down, so the
		//   sender of the channel for the response will come back as a disconnected error.
		// - the remote peer runs into an error while processing the request.
		//
		else if let Some( actor_id ) = self.relayed.get( &sid )
		{
			trace!( "{}, Incoming Call for relayed Actor", self.identify() );

			// The unwrap is safe, because we just checked self.relayed and we shall keep both
			// in sync
			//
			let mut relay = self.relays.get_mut( &actor_id ).unwrap().0.clone();


			self.exec.spawn( async move
			{
				let called = match relay.call( Call::new( frame ) ).await
				{
					// Peer for relay still online.
					//
					Ok(c) => c,

					// For now can only be mailbox closed.
					//
					Err(_) =>
					{
						let ctx = Peer::err_ctx( &self_addr, sid, None, "Process incoming Call to relay".to_string() );

						let err = ThesRemoteErr::RelayGone
						{
							ctx                     ,
							relay_id  : relay.id()  ,
							relay_name: relay.name(),
						};

						// If we are no longer around, just log the error.
						//
						if self_addr.send( RequestError::from( err.clone() ) ).await.is_err()
						{
							error!( "{}: {}.", self_addr.id(), &err );
						}

						return
					}
				};


				// TODO: Let the incoming remote know that their call failed
				// not sure the current process wants to know much about this. We sure shouldn't
				// crash.
				//
				match called
				{
					// Sending out over the sink (connection) didn't fail
					//
					Ok( receiver ) =>	{ match receiver.await
					{
						// The channel was not dropped before resolving, so the relayed connection didn't close
						// until we got a response.
						//
						Ok( result ) =>
						{
							match result
							{
								// The remote managed to process the message and send a result back
								// (no connection errors like deserialization failures etc)
								//
								Ok( resp ) =>
								{
									trace!( "{}: Got response from relayed call, sending out.", &identity );

									// This can fail if we are no longer connected, in which case there isn't much to do.
									//
									if self_addr.send( resp ).await.is_err()
									{
										error!( "{}: processing incoming call for relay: peer to client is closed before we finished sending a response to a request.", &identity );
									}

									return
								},

								// The relayed remote had errors while processing the request, such as deserialization.
								//
								Err(e) =>
								{
									let wire_format = Peer::prep_error( cid, &e );

									if self_addr.send( wire_format ).await.is_err()
									{
										error!( "{}: processing incoming call for relay: peer to client is closed before we finished sending a response to a request.", &identity );
									}

									return
								},
							}
						},


						// This can only happen if the sender got dropped. Eg, if the remote relay goes down
						// Inform peer that their call failed because we lost connection to the relay after
						// it was sent out.
						//
						Err(_) =>
						{
							let ctx = Self::err_ctx( &self_addr, sid, None, "Process incoming Call to relay".to_string() );

							let err = RequestError::from( ThesRemoteErr::RelayGone
							{
								ctx                     ,
								relay_id  : relay.id()  ,
								relay_name: relay.name(),
							});

							if self_addr.send( err ).await.is_err()
							{
								error!( "{}: processing incoming call for relay: peer to client is closed before we finished sending a response to a request.", &identity );
							}

							return
						}
					}},

					// Sending out call to relayed failed. This normally only happens if the connection
					// was closed, or a network transport malfunctioned.
					//
					Err(_) =>
					{
						let ctx = Peer::err_ctx( &self_addr, sid, None, "Process incoming Call to relay".to_string() );

						let err = RequestError::from( ThesRemoteErr::RelayGone
						{
							ctx                     ,
							relay_id  : relay.id()  ,
							relay_name: relay.name(),
						});

						if self_addr.send( err ).await.is_err()
						{
							error!( "{}: processing incoming call for relay: peer to client is closed before we finished sending a response to a request.", &identity );
						}

						return
					}
				}

			// TODO: Remove except
			//
			}).expect( "failed to spawn" );

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
