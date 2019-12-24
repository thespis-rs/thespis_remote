use crate::{ *, import::* } ;


/// Represents a type that exposes Services over the network. In the impl this is the Peer.
//
pub trait ServiceProvider: Actor
{
	/// Register a service map as the handler for service ids that might come in over the network
	//
	fn register_services( &mut self, sm: Arc< dyn ServiceMap + Send + Sync > );
}
