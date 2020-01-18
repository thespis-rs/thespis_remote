use { crate :: { peer::* }};

// Send this to the peer to tell it to start providing more services, or to change the service map that
// handles existing services.
//
#[ derive( Debug ) ]
//
pub struct AddServices
{
	/// The service map that will handle the services.
	/// This service map must include all the services when sm.services() is called.
	//
	pub sm: Arc<dyn ServiceMap>,
}

impl Message for AddServices { type Return = (); }



impl Handler<AddServices> for Peer
{
	fn handle( &mut self, msg: AddServices ) -> Return<'_, ()> { async move
	{
		trace!( "{}: handle AddServices", self.identify() );

		for sid in msg.sm.services()
		{
			self.services.insert( sid, msg.sm.clone() );
		}

	}.boxed() }
}
