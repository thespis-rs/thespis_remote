use crate :: { import::*, * };



/// Convenience trait specifying that some address can deliver both WireFormat and peer::Call messages.
//
pub trait Relay: Address<WireFormat, Error=ThesErr> + Address<Call, Error=ThesErr> + Identify + Send {}

impl<T> Relay for T where T: Address<WireFormat, Error=ThesErr> + Address<Call, Error=ThesErr> + Identify + Send {}


pub type RelayClosure = Box< dyn Fn( &ServiceID ) -> Box<dyn Relay> + Send>;


/// A wrapper type to be able to pass both an BoxAddress or a closure to RelayMap.
///
/// I considered using this for service_map_macro as well, but it's complicated to get right. Since that
/// get's the compiler to verify that a handler does in fact implement Handler for the type, we would have
/// to make this generic. However, since for relays we need it to have a handler that deals with both
/// WireFormat and Call, we would have to create some Eiter enum. That would still have been acceptable,
/// yet the both types have different return types as associated types in the Message impl. That means
/// that the return type would also have to be an enum. My gut says this is becoming to complex.
/// As an added down-side, the compiler can't map the Either Message type to the Either return type,
/// so we don't get compiler verification on that.
///
/// TODO: Further unifying remote and local handlers is desirable, but further thought is needed to find
///       an elegant solution. I don't consider it a priority because for local actors it's possible to
///       implement load balancing by having a proxy actor be the handler and letting that dispatch.
//
pub enum ServiceHandler
{
	/// A Box<dyn Address<Relay>>
	//
	Address( Box<dyn Relay> ),

	/// A closure that yields an Address.
	//
	Closure( RelayClosure ),
}



impl From< Box<dyn Relay> > for ServiceHandler
{
	fn from( addr: Box<dyn Relay> ) -> Self
	{
		ServiceHandler::Address( addr )
	}
}



impl From< RelayClosure > for ServiceHandler
{
	fn from( cl: RelayClosure ) -> Self
	{
		ServiceHandler::Closure( cl )
	}
}



// Would have been nice to have file and line number for the closure here, but it's rather hard to do.
//
impl fmt::Debug for ServiceHandler
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "ServiceHandler: " )?;

		match self
		{
			Self::Closure(_) => { write!( f, "Closure" )?; }

			Self::Address(a) =>
			{
				match a.name()
				{
					Some(n) => write!( f, "Address: id: {}, name: {:?}", a.id(), n )? ,
					None    => write!( f, "Address: id: {}", a.id()                )? ,
				}
			}
		};

		Ok(())
	}
}


impl fmt::Display for ServiceHandler
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		fmt::Debug::fmt( &self, f )
	}
}





