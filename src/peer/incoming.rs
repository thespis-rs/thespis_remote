use crate::{ import::*, * };

/// Type representing Messages coming in over the wire, for internal use only.
//
pub(super) struct Incoming<MS>
{
	pub msg: Result<MS, ThesRemoteErr>
}

impl<MS: 'static + MultiService + Send> Message for Incoming<MS>
{
	type Return = ();
}


/// Handler for incoming messages
//
impl<Out, MS> Handler<Incoming<MS>> for Peer<Out, MS>

	where Out: BoundsOut<MS>,
	      MS : BoundsMS     ,
{
fn handle( &mut self, incoming: Incoming<MS> ) -> ReturnNoSend<()>
{

Box::pin( async move
{
	let cid_null = <MS as MultiService>::ConnID::null();

	let frame = match incoming.msg
	{
		Ok ( mesg  ) => mesg,
		Err( error ) =>
		{
			error!( "Error extracting MultiService from stream: {:#?}", error );

			self.pharos.notify( &PeerEvent::Error( ConnectionError::Deserialize ) ).await;

			// Send an error back to the remote peer and close the connection
			//
			self.send_err( cid_null, &ConnectionError::Deserialize, true ).await;

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
	//
	// TODO: Error handling
	// --------------------
	//
	// A. When calling a remote process, it might send back an error instead of a response
	//
	// B. Incoming messages:
	//
	//    - might fail to deserialize
	//    - might be for an unknown actor
	//    - come in after we are closing down? Verify with the listen function. If there's no
	//      awaits between reception on the stream and here, then we can't see this condition in
	//      both places.
	//


	// I think these should never fail, because they accept random data in the current implementation.
	// However, since it's implementation dependant, and we are generic, we can't know that. It's probably
	// safer to assume that if these do fail we close the connection because all bet's are off for following
	// messages.
	//
	let sid = match frame.service()
	{
		Ok ( sid ) => sid,
		Err( err ) =>
		{
			error!( "Fail to get service_id from incoming frame: {}", err );

			self.pharos.notify( &PeerEvent::Error( ConnectionError::Deserialize ) ).await;

			// Send an error back to the remote peer and close the connection
			//
			self.send_err( cid_null, &ConnectionError::Deserialize, true ).await;

			return
		}
	};


	let cid = match frame.conn_id()
	{
		Ok ( cid ) => cid,
		Err( err ) =>
		{
			error!( "Fail to get conn_id from incoming frame: {}", err );

			self.pharos.notify( &PeerEvent::Error( ConnectionError::Deserialize ) ).await;

			// Send an error back to the remote peer and close the connection
			//
			self.send_err( cid_null, &ConnectionError::Deserialize, true ).await;

			return
		}
	};



	// It's a connection error from the remote peer
	//
	// This includes failing to deserialize our messages, failing to relay, unknown service, ...
	// TODO: when we sent a Call, it will have the cid in frame, so we should correctly react
	// to that and forward it to the original caller.
	//
	if sid.is_null()
	{
		if let Ok( err ) = serde_cbor::from_slice::<ConnectionError>( &frame.mesg() )
		{
			error!( "Remote error: {:?}", err );

			let shine = PeerEvent::RemoteError(err.clone());
			self.pharos.notify( &shine ).await;


			// Certain errors happen in the remote peer without it being able to close the
			// connection (because they happen in a spawned task). To work around this problem
			// for now, close them from this end. To really fix this, the spawend taskes in
			// sm.send_service and sm.call_service should have access to the addr of the peer,
			// but the methods are defined in a trait ServiceMap, and for now, there is no trait
			// for peers... so the iface does not have a way of referencing the type of the peer.
			//
			match err
			{
				  ConnectionError::InternalServerError
				| ConnectionError::Deserialize
				| ConnectionError::Serialize

				=>
				{
					if let Some( ref mut self_addr ) = self.addr
					{
						let _ = self_addr.send( CloseConnection{ remote: true } ).await;
					}
				},

				_ => {},
			}
		}
	}



	// it's an incoming send
	//
	else if cid.is_null()
	{
		trace!( "Incoming Send" );


		if let Some( typeid ) = self.services.get( &sid )
		{
			trace!( "Incoming Send for local Actor" );

			// We are keeping our internal state consistent, so the unwrap is fine. if it's in
			// self.services, it's in self.service_maps.
			//
			let sm = &mut self.service_maps.get( &typeid ).unwrap();


			match sm.send_service( frame )
			{
				Err(e) =>
				{
					error!( "Failed to send message to handler for service [{:?}]: {:?}", sid, e );

					match e.kind()
					{
						ThesRemoteErrKind::Deserialize(..) =>
						{
							self.pharos.notify( &PeerEvent::Error( ConnectionError::Deserialize ) ).await;

							// Send an error back to the remote peer and close the connection
							//
							self.send_err( cid_null, &ConnectionError::Deserialize, true ).await;
						},


						  ThesRemoteErrKind::ThesErr (..) // This is a spawn error
						| ThesRemoteErrKind::Downcast(..) =>
						{
							self.pharos.notify( &PeerEvent::Error( ConnectionError::InternalServerError ) ).await;

							// Send an error back to the remote peer and close the connection
							//
							self.send_err( cid_null, &ConnectionError::InternalServerError, true ).await;
						},


						ThesRemoteErrKind::UnknownService(..) =>
						{
							let err = ConnectionError::UnknownService(sid.into().to_vec());

							// Send an error back to the remote peer and don't close the connection
							//
							self.send_err( cid_null, &err, false ).await;

							let evt = PeerEvent::Error( err );
							self.pharos.notify( &evt ).await;

						},

						_ => {}
					}
				},

				Ok(_) => {}
			}
		}


		// service_id in self.relay => Send to recipient found in self.relay.
		//
		else if let Some( relay_id ) = self.relayed.get( &sid )
		{
			trace!( "Incoming Send for relayed Actor" );

			// We are keeping our internal state consistent, so the unwrap is fine. if it's in
			// self.relayed, it's in self.relays.
			//
			let relay = &mut self.relays.get_mut( &relay_id ).unwrap().0;

			// if this fails, well, the peer is no longer there. Warn remote and observers.
			// We let remote know we are no longer relaying to this service.
			// TODO: should we remove it from self.relayed? Normally there is detection mechanisms
			//       already that should take care of this, but then if they work, we shouldn't be here...
			//       also, if we no longer use unbounded channels, this might fail because the
			//       channel is full.
			//
			if  relay.send( frame ).await.is_err()
			{
				let err = ConnectionError::LostRelayBeforeSend( sid.into().to_vec() );
				error!( "Lost relay: {:?}", err );

				self.send_err( cid_null, &err, false ).await;

				let err = PeerEvent::Error( err );
				self.pharos.notify( &err ).await;
			};
		}


		// service_id unknown => send back and log error
		//
		else
		{
			error!( "Incoming Send for unknown Service: {}", &sid );

			// Send an error back to the remote peer and to the observers
			//
			let err = ConnectionError::UnknownService( sid.into().to_vec() );
			self.send_err( cid_null, &err, false ).await;

			let err = PeerEvent::Error( err );
			self.pharos.notify( &err ).await;
		}
	}


	// it's a response or an error
	//
	else if let Some( channel ) = self.responses.remove( &cid )
	{
		// It's a response
		//
		trace!( "Incoming Return" );

		// TODO: verify our error handling story here. Normally if this
		// fails it means the receiver of the channel was dropped... so
		// they are no longer interested in the reponse? Should we log?
		// Have a different behaviour in release than debug?
		//
		let _ = channel.send( frame );
	}


	// it's a call (!cid.is_null() and cid is unknown)
	//
	else
	{
		trace!( "Incoming Call" );


		if let Some( ref self_addr ) = self.addr
		{
			// It's a call for a local actor
			//
			if let Some( typeid ) = self.services.get( &sid )
			{
				trace!( "Incoming Call for local Actor" );

				// We are keeping our internal state consistent, so the unwrap is fine. if it's in
				// self.services, it's in self.service_maps.
				//
				let sm = &mut self.service_maps.get( &typeid ).unwrap();

				// Call actor
				//
				match sm.call_service( frame, self_addr.recipient() )
				{
					Err(e) =>
					{
						error!( "Failed to call handler for service [{:?}]: {:?}", sid, e );

						match e.kind()
						{

							ThesRemoteErrKind::Deserialize(..) =>
							{
								self.pharos.notify( &PeerEvent::Error( ConnectionError::Deserialize ) ).await;

								// Send an error back to the remote peer and close the connection
								//
								self.send_err( cid, &ConnectionError::Deserialize, true ).await;
							},


							ThesRemoteErrKind::UnknownService(..) =>
							{
								let err = ConnectionError::UnknownService(sid.into().to_vec());

								// Send an error back to the remote peer and don't close the connection
								//
								self.send_err( cid, &err, false ).await;

								let evt = PeerEvent::Error( err );
								self.pharos.notify( &evt ).await;

							},


							  ThesRemoteErrKind::ThesErr (..) // This is a spawn error
							| ThesRemoteErrKind::Downcast(..) =>
							{
								self.pharos.notify( &PeerEvent::Error( ConnectionError::InternalServerError ) ).await;

								// Send an error back to the remote peer and close the connection
								//
								self.send_err( cid, &ConnectionError::InternalServerError, true ).await;
							},


							_ => {}
						}
					},

					Ok(_) => {}
				}
			}


			// Call for relayed actor
			//
			// service_id in self.relay   => Create Call and send to recipient found in self.relay.
			//
			// What could possibly go wrong???
			// - relay peer has been shut down (eg. remote closed connection)
			// - we manage to call, but then when we await the response, the relay goes down, so the
			//   sender of the channel for the response will come back as a disconnected error.
			//
			else if let Some( peer ) = self.relayed.get( &sid )
			{
				trace!( "Incoming Call for relayed Actor" );

				// The unwrap is safe, because we just checked self.relayed and we shall keep both
				// in sync
				//
				let mut peer      = self.relays.get_mut( &peer ).unwrap().0.clone();
				let mut self_addr = self_addr.clone();

				rt::spawn( async move
				{
					// TODO: until we have bounded channels, this should never fail, so I'm leaving the expect.
					//
					let channel = peer.call( Call::new( frame ) ).await.expect( "Call to relay failed" );


					// TODO: Let the incoming remote know that their call failed
					// not sure the current process wants to know much about this. We sure shouldn't
					// crash.
					//
					match channel
					{
						Ok( receiver ) =>	{match  receiver.await
						{

							Ok( resp ) =>
							{
								trace!( "Got response from relayed call, sending out" );

								// TODO: until we have bounded channels, this should never fail, so I'm leaving the expect.
								//
								self_addr.send( resp ).await.expect( "Failed to send response from relay out on connection" );
							},


							// This can only happen if the sender got dropped. Eg, if the remote relay goes down
							// Inform peer that their call failed because we lost connection to the relay after
							// it was sent out.
							//
							Err( e ) =>
							{
								error!( "Lost relay: {:?}", e );

								// Send an error back to the remote peer
								//
								let err = Self::prep_error( cid, &ConnectionError::LostRelayBeforeResponse );


								// TODO: until we have bounded channels, this should never fail, so I'm leaving the expect.
								//
								self_addr.send( err ).await.expect( "send msg to self" );
							}
						}},

						// Sending out call by relay failed
						//
						Err( e ) =>
						{
							error!( "Lost relay: {:?}", e );

							// Send an error back to the remote peer
							//
							let err = Self::prep_error( cid, &ConnectionError::LostRelayBeforeCall );


							// TODO: until we have bounded channels, this should never fail, so I'm leaving the expect.
							//
							self_addr.send( err ).await.expect( "send msg to self" );
						}
					}

				}).expect( "failed to spawn" );

			}

			// service_id unknown => send back and log error
			//
			else
			{
				error!( "Incoming Call for unknown Actor: {}", sid );

				// Send an error back to the remote peer and to the observers
				//
				let err = ConnectionError::UnknownService( sid.into().to_vec() );
				self.send_err( cid_null, &err, false ).await;

				let err = PeerEvent::Error( err );
				self.pharos.notify( &err ).await;
			}
		}

		else
		{
			// we no longer have our address, we're shutting down. we can't really do anything
			// without our address we won't have the sink for the connection either. We can
			// no longer send outgoing messages
			//
			return
		}
	}

}) // End of Box::pin( async move

} // end of handle
} // end of impl Handler
