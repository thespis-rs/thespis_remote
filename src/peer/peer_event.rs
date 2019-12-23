use crate::{ import::*, * };


/// Events that can happen during the lifecycle of the peer. Use the [`observe`] method to subscribe to events.
///
/// When you see either `Closed` or `ClosedByRemote`, the connection is lost and you should drop all
/// addresses/recipients you hold for this peer, so it can be dropped. You can no longer send messages
/// over this peer after these events.
//
#[ derive( Debug, Clone, PartialEq ) ]
//
pub enum PeerEvent
{
	/// The connection is closed. It can no longer be used.
	//
	Closed,

	/// The remote endpoint closed the connection. It can no longer be used.
	//
	ClosedByRemote,

	/// A remote endpoint to which we relayed messages is no longer reachable.
	//
	RelayDisappeared(usize),

	/// An error happened while handling messages on the connection. These are
	/// generally errors that cannot be bubbled up because they are triggered when
	/// handling incoming messages, and thus are not triggered by a method call from
	/// client code. They are returned here out of band.
	//
	Error( ConnectionError ),

	/// The remote endpoint signals that they encountered an error while handling one of
	/// our messages.
	//
	RemoteError( ConnectionError ),
}


#[ derive( Debug, Clone ) ]
//
pub(super) struct RelayEvent
{
	pub(super) id : usize    ,
	pub(super) evt: PeerEvent,
}


impl Message for RelayEvent
{
	type Return = ();
}



/// Handler for events from provider connections.
/// If we notice Closed or ClosedByRemote on relays, we will stop relaying their services.
//
impl Handler<RelayEvent> for Peer
{
	fn handle( &mut self, re: RelayEvent ) -> Return< '_, <RelayEvent as Message>::Return >
	{
		match &self.addr
		{
			Some( a ) => trace!( "RelayEvent in peer: {}", a.id() ),
			None      => trace!( "RelayEvent in closing Peer"     ),
		}

		trace!( "Starting Handler<RelayEvent>, provider id: {:?}", &re );

		let peer_id = re.id;

		async move
		{
			match re.evt
			{
				// Clean up relays if they disappear
				//
				  PeerEvent::Closed
				| PeerEvent::ClosedByRemote =>
				{
					trace!( "Removing relay because it's connection is closed" );
					let cid_null = ConnID::null();


					// Remove all the relayed services from our relayed map.
					//
					let mut gone: Vec<ServiceID> = Vec::new();

					self.relayed.retain( |sid, peer|
					{
						let keep = *peer != peer_id;

						if !keep { gone.push( (*sid).clone() ) }

						keep
					});

					// Warn our remote that we are no longer providing these services
					//
					for sid in gone
					{
						let err = ConnectionError::ServiceGone( Into::<Bytes>::into( sid ).to_vec() );
						self.send_err( cid_null.clone(), &err, false ).await;
					}



					self.relays.remove( &re.id );

					let shine = PeerEvent::RelayDisappeared( re.id );
					self.pharos.send( shine ).await.expect( "pharos not closed" );
				}


				// Sometimes services might be no longer available. This can happen even if the connection
				// is not closed, because the relayed peer might be relaying other peers and only some
				// services disappear.
				//
				PeerEvent::RemoteError( ConnectionError::ServiceGone(sidvec) ) =>
				{
					let cid_null = ConnID::null();

					match ServiceID::try_from( Bytes::from( sidvec ) )
					{
						Err(_) => {},

						Ok(sid) =>
						{
							self.relayed.retain( |sid_stored, peer|
							{
								*peer != peer_id && **sid_stored == sid
							});

							// Warn our remote that we are no longer providing these services
							//
							let err = ConnectionError::ServiceGone( Into::<Bytes>::into( sid ).to_vec() );
							self.send_err( cid_null.clone(), &err, false ).await;
						}
					};
				}

				_ => {}
			}

			trace!( "End of handler RelayEvent" );

		}.boxed()
	}
}
