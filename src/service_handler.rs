use crate :: { import::*, * };



/// Convenience trait specifying that some address can deliver both WireFormat and peer::Call messages.
//
pub trait Relay: Address<WireFormat, Error=ThesErr> + Address<Call, Error=ThesErr> + Identify + Send + Sync {}

impl<T> Relay for T where T: Address<WireFormat, Error=ThesErr> + Address<Call, Error=ThesErr> + Identify + Send + Sync {}


pub type RelayClosure = Box< dyn Fn( &ServiceID ) -> Option<Box<dyn Relay + Send + Sync + Unpin + 'static >> + Send + Sync>;


/// A wrapper type to be able to pass both an BoxAddress or a closure to RelayMap.
///
/// TODO: Letting the closure return an option is misleading. It is an error, which will return internal
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




/// TODO:
///
//
impl fmt::Debug for ServiceHandler
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "dummy ServiceHandler" )
	}
}





