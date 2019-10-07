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
	Closed                         ,
	ClosedByRemote                 ,
	RelayDisappeared(usize)        ,
	Error      ( ConnectionError ) ,
	RemoteError( ConnectionError ) ,
}


#[ derive( Debug, Clone ) ]
//
pub(super) struct RelayEvent
{
	pub id : usize    ,
	pub evt: PeerEvent,
}


impl Message for RelayEvent
{
	type Return = ();
}



/// Handler for events from relays.
/// If we notice Closed or ClosedByRemote on relays, we will stop relaying their services.
//
impl<MS> Handler<RelayEvent> for Peer<MS> where MS: BoundsMS
{
	fn handle( &mut self, re: RelayEvent ) -> Return< <RelayEvent as Message>::Return >
	{
		let id = <Addr<Self> as Recipient<RelayEvent>>::actor_id( &self.addr.clone().unwrap() );

		trace!( "peer {:?}: starting Handler<RelayEvent>: {:?}", id , &re );

		let peer_id = re.id;

		Box::pin( async move
		{
			match re.evt
			{
				// Clean up relays if they disappear
				//
				  PeerEvent::Closed
				| PeerEvent::ClosedByRemote =>
				{
					trace!( "peer {:?}: Removing relay because it's connection is closed", id );
					let cid_null = <MS as MultiService>::ConnID::null();


					// Remove all the relayed services from our relayed map.
					//
					let mut gone: Vec<<MS as MultiService>::ServiceID> = Vec::new();

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
						let err = ConnectionError::ServiceGone( sid.into().to_vec() );
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
					let cid_null = <MS as MultiService>::ConnID::null();

					match <MS as MultiService>::ServiceID::try_from( Bytes::from( sidvec ) )
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
							let err = ConnectionError::ServiceGone( sid.into().to_vec() );
							self.send_err( cid_null.clone(), &err, false ).await;
						}
					};
				}

				_ => {}
			}

			trace!( "peer {:?}: End of handler relay event", id );
		})
	}
}
