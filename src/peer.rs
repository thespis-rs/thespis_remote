//! The peer module holds everything that deals with managing a remote connection over which
//! actor messages can be sent and received.
//
use crate :: { import::*, Codecs };


mod close_connection  ;
mod connection_error  ;
mod peer_event        ;
mod call              ;
mod incoming          ;
mod register_relay    ;

pub use call              :: Call             ;
pub use close_connection  :: CloseConnection  ;
pub use connection_error  :: ConnectionError  ;
pub use peer_event        :: PeerEvent        ;
pub use register_relay    :: RegisterRelay    ;
    use incoming          :: Incoming         ;
    use peer_event        :: RelayEvent       ;

// Reduce trait bound boilerplate, since we have to repeat them all over
//
/// Trait bounds for the stream of incoming messages
//
pub trait BoundsIn <MS: BoundsMS>: 'static + Stream< Item = Result<MS, ThesRemoteErr> > + Unpin + Send {}

/// Trait bounds for the Sink of outgoing messages.
//
pub trait BoundsOut<MS: BoundsMS>: 'static + Sink<MS, Error=ThesRemoteErr > + Unpin + Send {}

/// Trait bounds for the MultiService message format. This is the wire format for thespis_remote.
//
pub trait BoundsMS: 'static + Message<Return=()> + MultiService<CodecAlg=Codecs> + Send + fmt::Debug {}

impl<T, MS> BoundsIn<MS> for T

	where T : 'static + Stream< Item = Result<MS, ThesRemoteErr> > + Unpin + Send,
   	   MS: BoundsMS
{}

impl<T, MS> BoundsOut<MS> for T

	where T : 'static + Sink<MS, Error=ThesRemoteErr > + Unpin + Send,
	      MS: BoundsMS
{}

impl<T> BoundsMS for T
where T: 'static + Message<Return=()> + MultiService<CodecAlg=Codecs> + Send + fmt::Debug {}


/// Represents a connection to another process over which you can send actor messages.
///
/// ### Closing the connection
///
/// The reasoning behind a peer is that it is tied to a stream/sink, often a framed connection.
/// When the connection closes for whatever reason, the peer should dissappear and no longer handle
/// any messages.
///
/// If the remote closes the connection, and you are no longer holding any addresses to this
/// peer (or recipients for remote actors), then the peer will get dropped.
///
/// If you do hold recipients and try to send on them, 2 things can happen. Since Send is like
/// throwing a message in a bottle, without feedback, it's infallible, so your message will
/// just get dropped silently. If you use call, which returns a result, you will get an error
/// (ThesError::PeerSendAfterCloseConnection).
///
/// Peer uses the pharos crate to be observable over [`PeerEvent`]. This allows you to detect
/// when errors happen and to react accordingly. If the connection gets closed, you can make
/// reconnect and make a new peer.
///
/// ### Errors
/// A lot of things can go wrong with networking. The main issue is that we interprete the
/// MultiService messages that come in. They are actually still deserialized. The only validation
/// that has happened before is that the message size was at least HEADER_SIZE + 1, (in the codec).
///
/// We still deserialize elements of this message, and if things don't deserialize correctly it
/// might mean the stream is corrupt, and that what we think is the beginning of a message is not
/// actually the beginning. Peer has a conservative approach to this and will close the connection
/// as soon as a potential corruption has taken place. You will know this happens by observing the
/// event stream from `observe`.
///
/// Any errors that occur which do not hint that the stream has become corrupted will not close the
/// connection. When a connection is closed, just drop all addresses you hold to the peer to allow
/// it to be dropped, create a new connection and a new peer.
///
//
#[ derive( Actor ) ]
//
pub struct Peer<MS> where MS: BoundsMS
{
	/// The sink
	//
	outgoing      : Option< Box<dyn BoundsOut<MS> > >,

	/// This is needed so that the loop listening to the incoming stream can send messages to this actor.
	/// The loop runs in parallel of the rest of the actor, yet processing incoming messages need mutable
	/// access to our state, so we have to pass through a message, or we need to put everything in Rc<RefCell>>.
	/// For now, passing messages seems the cleaner solution.
	///
	/// It also allows us to hand out our address to things that have to respond to the remote on our connection.
	//
	addr          : Option< Addr<Self> >,

	/// The handle to the spawned listen function. If we drop this, the listen function immediately stops.
	//
	listen_handle : Option< RemoteHandle<()>>,

	/// Information required to process incoming messages. The first element is a boxed Receiver, and the second is
	/// the service map that takes care of this service type.
	//
	// The error type here needs to correspond to the error type of the recipient we are going to pass
	// to `Servicemap::call_service`. TODO: In principle we should be generic over recipient type, but for now
	// I have put ThesErr, because it's getting to complex.
	//
	services      : HashMap<&'static <MS as MultiService>::ServiceID, TypeId>,
	service_maps  : HashMap<TypeId, BoxServiceMap<MS> >,

	/// All services that we relay to another peer. It has to be of the same type for now since there is
	/// no trait for peers.
	///
	/// We store a map of the sid to the actor_id and then a map from actor_id to both addr and
	/// remote handle for the PeerEvents.
	///
	/// These two fields should be kept in sync. Eg, we call unwrap on the get_mut on relays if
	/// we found the id in relayed.
	//
	relayed       : HashMap< &'static <MS as MultiService>::ServiceID, usize >,
	relays        : HashMap< usize, (Addr<Self>, RemoteHandle<()>)           >,

	/// We use onshot channels to give clients a future that will resolve to their response.
	//
	responses     : HashMap< <MS as MultiService>::ConnID, oneshot::Sender<Result<MS, ConnectionError>> >,

	/// The pharos allows us to have observers.
	//
	pharos        : Pharos<PeerEvent>,
}



impl<MS> Peer<MS> where MS : BoundsMS,
{
	/// Create a new peer to represent a connection to some remote.
	//
	pub fn new( addr: Addr<Self>, mut incoming: impl BoundsIn<MS>, outgoing: impl BoundsOut<MS> ) -> Result< Self, ThesRemoteErr >
	{
		trace!( "create peer" );

		// Hook up the incoming stream to our address.
		//
		let mut addr2 = addr.clone();

		let listen = async move
		{
			// This can fail if:
			// - channel is full (TODO: currently we use unbounded, so that won't happen, but it might
			//   use unbounded amounts of memory.)
			// - the receiver is dropped. The receiver is our mailbox, so it should never be dropped
			//   as long as we have an address to it.
			//
			// So, I think we can unwrap for now.
			//
			while let Some(msg) = incoming.next().await
			{
				trace!( "incoming message" );
				addr2.send( Incoming{ msg } ).await.expect( "peer: send incoming msg to self" )
			}

			// Same as above.
			//
			addr2.send( CloseConnection{ remote: true } ).await.expect( "peer send to self");
		};

		// When we need to stop listening, we have to drop this future, because it contains
		// our address, and we won't be dropped as long as there are adresses around.
		//
		let (remote, handle) = listen.remote_handle();
		rt::spawn( remote ).context( ThesRemoteErrKind::ThesErr( "Incoming stream for peer".into() ))?;

		Ok( Self
		{
			outgoing     : Some( Box::new(outgoing) ) ,
			addr         : Some( addr )               ,
			responses    : HashMap::new()             ,
			services     : HashMap::new()             ,
			service_maps : HashMap::new()             ,
			relayed      : HashMap::new()             ,
			relays       : HashMap::new()             ,
			listen_handle: Some( handle )             ,
			pharos       : Pharos::default()          ,
		})
	}



	/// Tell this peer to make a given service avaible to a remote, by forwarding incoming requests to the given peer.
	/// For relaying services from other processes.
	//
	pub fn register_relayed_services
	(
		&mut self                                                        ,
		     services    : Vec<&'static <MS as MultiService>::ServiceID> ,
		     peer        : Addr<Self>                                    ,
		     peer_events : Events<PeerEvent>                             ,

	) -> Result<(), ThesRemoteErr>

	{
		trace!( "peer: starting Handler<RegisterRelay<MS>>" );

		// When called from a RegisterRelay message, it's possible that in the mean time
		// the connection closed. We should immediately return a ConnectionClosed error.
		//
		let mut self_addr = match &self.addr
		{
			Some( self_addr ) => self_addr.clone() ,
			None              =>

				Err( ThesRemoteErrKind::ConnectionClosed( "register_relayed_services".to_string() ))?,
		};

		let peer_id = < Addr<Self> as Recipient<RelayEvent> >::actor_id( &peer );


		let listen = async move
		{
			// We need to map this to a custom type, since we had to impl Message for it.
			//
			let stream = &mut peer_events.map( |evt| RelayEvent{ id: peer_id, evt } );

			// This can fail if:
			// - channel is full (for now we use unbounded)
			// - the receiver is dropped. The receiver is our mailbox, so it should never be dropped
			//   as long as we have an address to it.
			//
			// So, I think we can unwrap for now.
			//
			self_addr.send_all( stream ).await.expect( "peer send to self" );

			// Same as above.
			// Normally relays shouldn't just dissappear, without notifying us, but it could
			// happen for example that the peer already shut down and the above stream was already
			// finished, we would immediately be here, so we do need to clean up.
			// Since we are doing multi threading it's possible to receive the peers address,
			// but it's no longer valid. So send ourselves a message.
			//
			let evt = PeerEvent::Closed;

			self_addr.send( RelayEvent{ id: peer_id, evt } ).await.expect( "peer send to self");
		};

		// When we need to stop listening, we have to drop this future, because it contains
		// our address, and we won't be dropped as long as there are adresses around.
		//
		let (remote, handle) = listen.remote_handle();
		rt::spawn( remote ).map_err( |_|
		{
			ThesRemoteErrKind::ThesErr( "Stream of events from relay peer".into() )
		})?;

		self.relays .insert( peer_id, (peer, handle) );

		for sid in services
		{
			trace!( "Register relaying to: {}", sid );
			self.relayed.insert( sid, peer_id );
		}

		Ok(())
	}



	// actually send the message accross the wire
	//
	async fn send_msg( &mut self, msg: MS ) -> Result<(), ThesRemoteErr>
	{
		match &mut self.outgoing
		{
			Some( out ) =>  out.send( msg ).await,

			None =>
			{
				Err( ThesRemoteErrKind::ConnectionClosed( "send MultiService over network".to_string() ))?
			}
		}
	}



	// actually send the error accross the wire. This is for when errors happen on receiving
	// messages (eg. Deserialization errors).
	//
	async fn send_err<'a>
	(
		&'a mut self                                ,
		     cid  : <MS as MultiService>::ConnID ,
		     err  : &'a ConnectionError             ,

		     // whether the connection should be closed (eg stream corrupted)
		     //
		     close: bool                         ,
	)
	{
		if let Some( ref mut out ) = self.outgoing
		{
			let msg = Self::prep_error( cid, err );

			let _ = out.send( msg ).await;

			if close {
			if let Some( ref mut addr ) = self.addr
			{
				// until we have bounded channels, this should never fail, so I'm leaving the expect.
				//
				addr.send( CloseConnection{ remote: false } ).await.expect( "send close connection" );
			}}
		}
	}



	// serialize a ConnectionError to be sent across the wire. Public because used in
	// sevice_map macro.
	//
	#[ doc( hidden ) ]
	//
	pub fn prep_error( cid: <MS as MultiService>::ConnID, err: &ConnectionError ) -> MS
	{
		let serialized   = serde_cbor::to_vec( err ).expect( "serialize response" );
		let codec: Bytes = Codecs::CBOR.into();

		let codec2 = match <MS as MultiService>::CodecAlg::try_from( codec )
		{
			Ok ( c ) => c,
			Err( _ ) => panic!( "Failed to create codec from bytes" ),
		};

		// sid null is the marker that this is an error message.
		//
		MS::create
		(
			<MS as MultiService>::ServiceID::null() ,
			cid                                     ,
			codec2                                  ,
			serialized.into()                       ,
		)
	}
}



// Put an outgoing multiservice message on the wire.
// TODO: why do we not return the error?
//
impl<MS> Handler<MS> for Peer<MS> where MS: BoundsMS,
{
	fn handle( &mut self, msg: MS ) -> Return<'_, ()>
	{
		Box::pin( async move
		{
			trace!( "Peer sending OUT" );

			let _ = self.send_msg( msg ).await;

		})
	}
}



// Pharos, shine!
//
impl<MS> Observable<PeerEvent> for Peer<MS> where MS: BoundsMS
{
	type Error = pharos::Error;

	/// Register an observer to receive events from this connection. This will allow you to detect
	/// Connection errors and loss. Note that the peer automatically goes in shut down mode if the
	/// connection is closed. When that happens, you should drop all remaining addresses of this peer.
	/// An actor does not get dropped as long as you have adresses to it.
	///
	/// You can then create a new connection, frame it, and create a new peer. This will send you
	/// a PeerEvent::Closed if the peer is in unsalvagable state and you should drop all addresses
	///
	/// See [PeerEvent] for more details on all possible events.
	//
	fn observe( &mut self, config: ObserveConfig<PeerEvent> ) -> Result< Events<PeerEvent>, pharos::Error >
	{
		self.pharos.observe( config )
	}
}

impl<MS> ServiceProvider<MS> for Peer<MS> where MS: BoundsMS
{
	/// Register a service map as the handler for service ids that come in over the network. Normally you should
	/// not call this directly, but use [´thespis_iface_remote::ServiceMap::register_with_peer´].
	//
	fn register_services( &mut self, services: &[&'static <MS as MultiService>::ServiceID], sm: BoxServiceMap<MS> )
	{
		let id = sm.type_id();

		self.service_maps.insert( id, sm );


		for sid in services.iter()
		{
			trace!( "Register Service: {:?}", sid );
			self.services.insert( sid, id );
		}
	}
}



impl<MS> fmt::Debug for Peer<MS> where MS: BoundsMS
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "Peer" )
	}
}


impl<MS> Drop for Peer<MS> where MS: BoundsMS
{
	fn drop( &mut self )
	{
		let mut id = String::new();

		if let Some( ref addr ) = self.addr
		{
			id = format!( ": {:?}", addr );
		}

		trace!( "Drop peer{:?}", id );
	}
}
