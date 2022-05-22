use crate::{ import::*, PeerErr, PeerErrCtx, ServiceID, ThesWF, peer::Response } ;


/// This interface is what the Peer type uses to deliver messages. An implementation is provided
/// for you in the `service_map` macro. You can however roll your own.
///
/// RelayMap also implements this for relaying certain services to remote providers.
///
/// The crux is that a service map returns a future that processes the requests and returns
/// whatever is relevant to the remote client.
//
pub trait ServiceMap<Wf = ThesWF>: fmt::Debug + Send + Sync
{
	/// Send a message to a handler. This should take care of deserialization.
	//
	fn send_service( &self, msg: Wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	;


	/// Call a Service.
	/// This should take care of deserialization. The return address is the address of the peer
	/// to which the serialized answer shall be send.
	//
	fn call_service( &self, msg: Wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	;


	/// Get a list of all services provided by this service map.
	//
	// TODO: Find a way to avoid the heap allocation.
	//
	fn services( &self ) -> Box<dyn Iterator<Item = &ServiceID> + '_ >;
}
