use crate :: { import::*, *, peer::request_error::RequestError };
use std::ops::Deref;


/// Convenience trait specifying that some address can deliver both WireFormat and peer::Call messages.
//
pub trait Relay: Address<WireFormat, Error=ThesErr> + Address<Call, Error=ThesErr> + Send {}

impl<T> Relay for T where T: Address<WireFormat, Error=ThesErr> + Address<Call, Error=ThesErr> + Send {}


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
	// TODO: - what about taking but one closure that returns the correct handler
	//       - closure vs fn pointer, Fn, FnOnce or FnMut?
	//       - what about inherently concurrent hashmaps like evmap, dashmap?
	//
	handlers: RwLock<HashMap< ServiceID, Box< dyn Fn( &ServiceID ) -> Option<Box<dyn Relay>> + Send + Sync> >>
}


impl RelayMap
{
	/// Create a RelayMap.
	//
	pub fn new() -> Self
	{
		Self { handlers: RwLock::new( HashMap::new() ) }
	}


	/// Register a handler for a given sid.
	//
	pub fn register_handler( &self, sid: ServiceID, handler: Box< dyn Fn( &ServiceID ) -> Option<Box<dyn Relay>> + Send + Sync> )
	{
		self.handlers.write().insert( sid, handler );
	}


	async fn handle_err( mut peer: Addr<Peer>, err: ThesRemoteErr )
	{
		if peer.send( RequestError::from( err.clone() ) ).await.is_err()
		{
			error!
			(
				"Peer ({}, {:?}): Processing incoming call: peer to client is closed, but processing request errored on: {}.",
				peer.id(),
				Address::<WireFormat>::name( &peer ),
				&err
			);
		}
	}
}



impl ServiceMap for RelayMap
{
	/// Send a message to a handler. This should take care of deserialization.
	//
	fn send_service( &self, msg: WireFormat, peer: Addr<Peer> ) -> Return<'static, ()>
	{
		trace!( "RelayMap: Incoming Send for relayed actor." );

		let sid = msg.service();

		// This sid should be in our map.
		//
		let mut addr = match self.handlers.read().get( &sid ).map( |fnp| fnp( &sid ) ).flatten()
		{
			Some(x) => x,

			None =>
			{
				let ctx = Peer::err_ctx( &peer, sid, None, "Process send to relayed Actor".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::NoHandler{ ctx } ).boxed()
			}
		};


		async move
		{
			if addr.send( msg ).await.is_err()
			{
				let ctx = Peer::err_ctx( &peer, sid.clone(), None, "Process send to relayed Actor".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::HandlerDead{ ctx } ).await;
			}

		}.boxed()

	}


	/// Call a Service.
	/// This should take care of deserialization. The return address is the address of the peer
	/// to which the serialized answer shall be send.
	//
	fn call_service( &self, frame: WireFormat, mut peer: Addr<Peer> ) -> Return<'static, ()>
	{
		trace!( "RelayMap: Incoming Call for relayed actor." );

		let sid = frame.service();
		let cid = frame.conn_id();


		// This sid should be in our map.
		//
		let mut relay = match self.handlers.read()

			.get( &sid ).map( |fnp| fnp( &sid ) ).flatten()

		{
			Some(x) => x,

			None =>
			{
				let ctx = Peer::err_ctx( &peer, sid, cid, "Process call for relayed Actor".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::NoHandler{ ctx } ).boxed()
			}
		};


		async move
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
					let ctx = Peer::err_ctx( &peer, sid, cid, "Process incoming Call to relay".to_string() );

					let err = ThesRemoteErr::RelayGone
					{
						ctx                          ,
						relay_id  : Address::<Call>::actor_id( relay.deref() ),
						relay_name: Address::<Call>::name    ( relay.deref() ),
					};

					// If we are no longer around, just log the error.
					//
					if peer.send( RequestError::from( err.clone() ) ).await.is_err()
					{
						error!( "Peer {}: {}.", peer.id(), &err );
					}

					return
				}
			};



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
								trace!( "Peer {}: Got response from relayed call, sending out.", &peer.id() );

								// This can fail if we are no longer connected, in which case there isn't much to do.
								//
								if peer.send( resp ).await.is_err()
								{
									error!( "Peer {}: processing incoming call for relay: peer to client is closed before we finished sending a response to the request.", &peer.id() );
								}

								return
							},

							// The relayed remote had errors while processing the request, such as deserialization.
							//
							Err(e) =>
							{
								let wire_format = Peer::prep_error( cid, &e );

								// Just forward the error to the client.
								//
								if peer.send( wire_format ).await.is_err()
								{
									error!( "Peer {}: processing incoming call for relay: peer to client is closed before we finished sending a response to the request.", &peer.id() );
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
						let ctx = Peer::err_ctx( &peer, sid, cid, "Process incoming Call to relay".to_string() );

						let err = RequestError::from( ThesRemoteErr::RelayGone
						{
							ctx                                                   ,
							relay_id  : Address::<Call>::actor_id( relay.deref() ),
							relay_name: Address::<Call>::name    ( relay.deref() ),
						});

						if peer.send( err ).await.is_err()
						{
							error!( "Peer {}: processing incoming call for relay: peer to client is closed before we finished sending a response to a request.", &peer.id() );
						}

						return
					}
				}},

				// Sending out call to relayed failed. This normally only happens if the connection
				// was closed, or a network transport malfunctioned.
				//
				Err(_) =>
				{
					let ctx = Peer::err_ctx( &peer, sid, cid, "Process incoming Call to relay".to_string() );

					let err = RequestError::from( ThesRemoteErr::RelayGone
					{
						ctx                                                   ,
						relay_id  : Address::<Call>::actor_id( relay.deref() ),
						relay_name: Address::<Call>::name    ( relay.deref() ),
					});

					if peer.send( err ).await.is_err()
					{
						error!( "Peer {}: processing incoming call for relay: peer to client is closed before we finished sending a response to a request.", &peer.id() );
					}

					return
				}
			}

		}.boxed()
	}

	// We need to make a Vec here because the hashmap.keys() doesn't have a static lifetime.
	//
	fn services( &self ) -> Vec<ServiceID>
	{
		let handlers = self.handlers.read();

		let mut s: Vec<ServiceID> = Vec::with_capacity( handlers.len() );

		for sid in handlers.keys()
		{
			s.push( sid.clone() );
		}

		s
	}
}




/// TODO: Print actor name if it has one
///
/// Will print something like:
///
/// ```ignore
/// RelayMap
/// {
///    Add  - sid: c5c22cab4f2d334e0000000000000000 - handler (actor_id): 0
///    Show - sid: 0617b1cfb55b99700000000000000000 - handler (actor_id): 0
/// }
/// ```
//
impl fmt::Debug for RelayMap
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "dummy RelayMap" )
	}
}
