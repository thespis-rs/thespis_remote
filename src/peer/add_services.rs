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
	pub sm: Box<dyn ServiceMap>,
}

impl Message for AddServices { type Return = Result<(), ThesRemoteErr>; }



impl Handler<AddServices> for Peer
{
	#[async_fn] fn handle( &mut self, msg: AddServices ) -> Result<(), ThesRemoteErr>
	{
		trace!( "{}: handle AddServices", self.identify() );

		self.register_services( msg.sm ).await?;

		Ok(())
	}
}
