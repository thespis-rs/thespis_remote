use { crate :: { peer::* }};

// Send this to the peer to tell it to start providing more services, or to change the service map that
// handles existing services.
//
#[ derive( Debug ) ]
//
pub struct RemoveServices
{
	/// The services to provide.
	//
	pub services: Vec<ServiceID>,
}

impl Message for RemoveServices { type Return = (); }



impl Handler<RemoveServices> for Peer
{
	fn handle( &mut self, msg: RemoveServices ) -> Return<'_, ()> { async move
	{
		trace!( "{}: handle RemoveServices", self.identify() );

		for sid in msg.services
		{
			self.services.remove( &sid );
		}

	}.boxed() }
}
