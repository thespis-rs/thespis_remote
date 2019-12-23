use crate :: { import::*, * };


/// Type for registering services to be relayed.
///
/// MS must be of the same type as the type parameter on [Peer].
//
#[ derive( Debug ) ]
//
pub struct RegisterRelay
{
	/// The services that are to be relayed
	//
	pub services: Vec<&'static ServiceID> ,

	/// The peer to which to relay the selected services them.
	//
	pub peer: Addr< Peer >,

	/// The events stream for the relay peer. Used to detect errors and connection close.
	//
	pub peer_events: Events<PeerEvent>,
}

impl Message for RegisterRelay
{
	type Return = Result<(), ThesRemoteErr>;
}



/// Handler for RegisterRelay
///
/// Tell this peer to make a given service avaible to a remote, by relaying incoming requests to the given peer.
/// For relaying services from other processes. You should normally use the method [`register_relayed_services`]
/// rather than sending this message, so that your peer is completely set up before starting it's mailbox. However
/// it can happen that the connection to the relay is lost, but you want to reconnect at runtime and resume relaying,
/// so this is provided for that scenario, since you won't be able to call [`register_relayed_services`] once your
/// peer has been started.
///
/// TODO: - verify we can relay services unknown at compile time. Eg. could a remote process ask in runtime
///       could you please relay for me. We just removed a type parameter here, which should help, but we
///       need to test it to make sure it works.
///
// Design:
// - take a peer with a vec of services to relay over that peer.
// - store in a hashmap, but put the peer address in an Rc? + a unique id (addr doesn't have Eq)
//
//
impl Handler< RegisterRelay > for Peer
{
	fn handle( &mut self, msg: RegisterRelay ) -> Return< '_, <RegisterRelay as Message>::Return >
	{
		trace!( "peer: starting Handler<RegisterRelay>" );

		let RegisterRelay { services, peer, peer_events } = msg;

		Box::pin( async move
		{
			self.register_relayed_services( services, peer, peer_events )
		})
	}
}
