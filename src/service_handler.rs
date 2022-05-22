use crate :: { import::*, * };



/// Convenience trait specifying that some address can deliver both the wire format and peer::Call messages.
//
pub trait Relay<Wf>

	where Wf  : WireFormat,
	      Self: Address<Wf, Error=ThesErr> + Address<Call<Wf>, Error=ThesErr> + Send,
{}

impl<T, Wf> Relay<Wf> for T

	where Wf  : WireFormat,
	      Self: Address<Wf, Error=ThesErr> + Address<Call<Wf>, Error=ThesErr> + Send,
{}


pub type RelayClosure<Wf> = Box< dyn Fn( &ServiceID ) -> Box<dyn Relay<Wf>> + Send>;


/// A wrapper type to be able to pass both an BoxAddress or a closure to RelayMap.
///
/// I considered using this for service_map_macro as well, but it's complicated to get right. Since that
/// get's the compiler to verify that a handler does in fact implement Handler for the type, we would have
/// to make this generic. However, since for relays we need it to have a handler that deals with both
/// CborWF and Call, we would have to create some Either enum. That would still have been acceptable,
/// yet the both types have different return types as associated types in the Message impl. That means
/// that the return type would also have to be an enum. My gut says this is becoming to complex.
/// As an added down-side, the compiler can't map the Either Message type to the Either return type,
/// so we don't get compiler verification on that.
///
/// TODO: Further unifying remote and local handlers is desirable, but further thought is needed to find
///       an elegant solution. I don't consider it a priority because for local actors it's possible to
///       implement load balancing by having a proxy actor be the handler and letting that dispatch.
//
pub enum ServiceHandler<Wf>
{
	/// A Box<dyn Address<Relay>>
	//
	Address( Box<dyn Relay<Wf>> ),

	/// A closure that yields an Address.
	//
	Closure( RelayClosure<Wf> ),
}



impl<Wf> From< Box<dyn Relay<Wf>> > for ServiceHandler<Wf>
{
	fn from( addr: Box<dyn Relay<Wf>> ) -> Self
	{
		ServiceHandler::Address( addr )
	}
}



impl<Wf> From< RelayClosure<Wf> > for ServiceHandler<Wf>
{
	fn from( cl: RelayClosure<Wf> ) -> Self
	{
		ServiceHandler::Closure( cl )
	}
}



// Would have been nice to have file and line number for the closure here, but it's rather hard to do.
//
impl<Wf> fmt::Debug for ServiceHandler<Wf>
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


impl<Wf> fmt::Display for ServiceHandler<Wf>
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		fmt::Debug::fmt( &self, f )
	}
}





