use crate::{ import::*, PeerErr, PeerErrCtx, ServiceID, CborWF, peer::Response } ;


/// This interface is what the Peer type uses to deliver messages. An implementation is provided
/// for you in the `service_map` macro. You can however roll your own.
///
/// RelayMap also implements this for relaying certain services to remote providers.
///
/// The crux is that a service map returns a future that processes the requests and returns
/// whatever is relevant to the remote client.
//
pub trait ServiceMap<Wf = CborWF>: fmt::Debug + Send + Sync
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



	/// Whether the peer should apply backpressure for this service map. Generally true when locally processed
	/// and false for relayed messages.
	///
	/// Note that we only know which service map a message is for when we already reserved a slot on the semaphore,
	/// so if the peer is backed up, this can still stop a request for this service map from getting in.
	/// However if this returns false, as soon as the service map is known, the slot will be released.
	//
	fn apply_backpressure( &self ) -> bool;
}
