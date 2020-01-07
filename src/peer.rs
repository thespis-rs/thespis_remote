//! The peer module holds everything that deals with managing a remote connection over which
//! actor messages can be sent and received.
//
use crate :: { import::*, * };


    mod close_connection  ;
    mod connection_error  ;
    mod peer_event        ;
    mod call              ;
    mod incoming          ;
    mod register_relay    ;
pub mod request_error     ;

pub use call              :: Call             ;
pub use close_connection  :: CloseConnection  ;
pub use connection_error  :: ConnectionError  ;
pub use peer_event        :: PeerEvent        ;
pub use register_relay    :: RegisterRelay    ;
    use incoming          :: Incoming         ;
    use peer_event        :: RelayEvent       ;
    use request_error     :: RequestError     ;


// Reduce trait bound boilerplate, since we have to repeat them all over
//
/// Trait bounds for the stream of incoming messages
//
pub trait BoundsIn : 'static + Stream< Item = Result<WireFormat, ThesRemoteErr> > + Unpin + Send {}

/// Trait bounds for the Sink of outgoing messages.
//
pub trait BoundsOut: 'static + Sink<WireFormat, Error=ThesRemoteErr > + Unpin + Send {}


impl<T> BoundsIn for T

	where T : 'static + Stream< Item = Result<WireFormat, ThesRemoteErr> > + Unpin + Send
{}

impl<T> BoundsOut for T

	where T : 'static + Sink<WireFormat, Error=ThesRemoteErr > + Unpin + Send
{}



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
pub struct Peer
{
	/// The sink
	//
	outgoing      : Option< Box<dyn BoundsOut> >,

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
	services      : HashMap<&'static ServiceID, TypeId>,
	service_maps  : HashMap<TypeId, Arc<dyn ServiceMap + Send + Sync > >,

	/// All services that we relay to another peer. It has to be of the same type for now since there is
	/// no trait for peers.
	///
	/// We store a map of the sid to the actor_id and then a map from actor_id to both addr and
	/// remote handle for the PeerEvents.
	///
	/// These two fields should be kept in sync. Eg, we call unwrap on the get_mut on relays if
	/// we found the id in relayed.
	///
	//
	relayed       : HashMap< &'static ServiceID, usize >,
	relays        : HashMap< usize, (Addr<Self>, RemoteHandle<()>) >,

	/// We use onshot channels to give clients a future that will resolve to their response.
	//
	responses     : HashMap< ConnID, oneshot::Sender<Result<WireFormat, ConnectionError>> >,

	/// The pharos allows us to have observers.
	//
	pharos        : Pharos<PeerEvent>,

	// An executor to spawn tasks, for processing requests.
	// TODO: make this an Arc.
	//
	exec          : Box< dyn Spawn + Send + Sync + 'static >,
}



impl Peer
{
	/// Create a new peer to represent a connection to some remote.
	/// `addr` is the actor address for this actor.
	//
	pub fn new
	(
		    addr    : Addr<Self>                         ,
		mut incoming: impl BoundsIn                      ,
		    outgoing: impl BoundsOut                     ,
		    exec    : impl Spawn + Send + Sync + 'static ,
	)

		-> Result< Self, ThesRemoteErr >

	{
		trace!( "Create peer" );

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
				trace!( "Peer ({}{}): incoming message.", addr2.id(), addr2.string_name() );
				addr2.send( Incoming{ msg } ).await.expect( "peer: send incoming msg to self" )
			}

			trace!( "Peer ({}{}):  incoming stream end, closing out.", addr2.id(), addr2.string_name() );

			// Same as above.
			//
			addr2.send( CloseConnection{ remote: true } ).await.expect( "peer send to self");
		};

		// When we need to stop listening, we have to drop this future, because it contains
		// our address, and we won't be dropped as long as there are adresses around.
		//
		let (remote, handle) = listen.remote_handle();

		exec.spawn( remote ).map_err( |e| -> ThesRemoteErr
		{
			ThesErr::Spawn{ actor: format!( "Incoming stream for peer: {}", e ) }.into()

		})?;


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
			exec         : Box::new( exec )           ,
		})
	}



	#[ cfg( feature = "futures_codec" ) ]
	//
	/// Create a Peer directly from an asynchronous stream. This is a convenience wrapper around Peer::new so
	/// you don't have to bother with framing the connection.
	///
	/// *addr*: This peers own adress.
	///
	/// *socket*: The async stream to frame.
	///
	/// *max_size*: The maximum accepted message size in bytes. The codec will reject parsing a message from the
	/// stream if it exceeds this size. Also used for encoding outgoing messages.
	/// **Set the same max_size in the remote!**.
	//
	pub fn from_async_read
	(
		addr    : Addr<Self>                                                 ,
		socket  : impl FutAsyncRead + FutAsyncWrite + Unpin + Send + 'static ,
		max_size: usize                                                      ,
		exec    : impl Spawn + Send + Sync + 'static                         ,
	)

		-> Result< Self, ThesRemoteErr >

	{
		let codec = ThesCodec::new(max_size);

		let (sink, stream) = FutFramed::new( socket, codec ).split();

		Peer::new( addr.clone(), stream, sink, exec )
	}




	/// Register a service map as the handler for service ids that come in over the network. Normally you should
	/// not call this directly, but use [´thespis_remote::ServiceMap::register_with_peer´].
	//
	pub fn register_services( &mut self, sm: Arc< dyn ServiceMap + Send + Sync > )
	{
		let id = sm.type_id();

		for sid in sm.services().iter()
		{
			trace!( "{}: Register Service: {:?}", self.identify(), &sid );
			self.services.insert( sid, id );
		}

		self.service_maps.insert( id, sm );
	}



	/// Tell this peer to make a given service avaible to a remote, by forwarding incoming requests to the given
	/// providing peer (connection to a remote provider).
	/// For relaying services from other processes.
	//
	pub fn register_relayed_services
	(
		&mut self                                  ,
		     services    : Vec<&'static ServiceID> ,

		     // TODO: provider might be a different type then Self?
		     //
		     provider    : Addr<Self>              ,
		     peer_events : Events<PeerEvent>       ,

	) -> Result<(), ThesRemoteErr>

	{
		let identity = self.identify();

		trace!( "{}: starting register_relayed_services", &identity );

		// When called from a RegisterRelay message, it's possible that in the mean time
		// the connection closed. We should immediately return a ConnectionClosed error.
		//
		let mut self_addr = match &self.addr
		{
			Some( addr ) => addr.clone(),

			None =>
			{
				let ctx = ErrorContext
				{
					context  : Some( "register_relayed_services".to_string() ),
					peer_id  : None ,
					peer_name: None ,
					sid      : None ,
					cid      : None ,
				};

				return Err( ThesRemoteErr::ConnectionClosed{ ctx } )
			}
		};


		let provider_id = provider.id();
		let identity2   = identity.clone();

		let listen = async move
		{
			// We need to map this to a custom type, since we had to impl Message for it.
			//
			let stream = &mut peer_events.map( |evt| Ok( RelayEvent{ id: provider_id, evt } ) );

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

			self_addr.send( RelayEvent{ id: provider_id, evt } ).await.expect( "peer send to self");

			trace!( "{}: Stop listening to relay provider events: peer", &identity2 );
		};

		// When we need to stop listening, we have to drop this future, because it contains
		// our address, and we won't be dropped as long as there are adresses around.
		//
		let (remote, handle) = listen.remote_handle();

		self.exec.spawn( remote ).map_err( |_| -> ThesRemoteErr
		{
			ThesErr::Spawn{ actor: "Stream of events from relay peer".to_string() }.into()
		})?;

		self.relays.insert( provider_id, (provider, handle) );

		for sid in services
		{
			trace!( "{}: Register relaying to: {}, provided by {}", &identity, sid, provider_id );
			self.relayed.insert( sid, provider_id );
		}

		Ok(())
	}



	// actually send the message accross the wire
	//
	async fn send_msg( &mut self, msg: WireFormat ) -> Result<(), ThesRemoteErr>
	{
		trace!( "{}: sending OUT WireFormat", self.identify() );

		match &mut self.outgoing
		{
			Some( out ) => out.send( msg ).await,

			None =>
			{
				let ctx = ErrorContext
				{
					context  : Some( "register_relayed_services".to_string() ),
					peer_id  : None ,
					peer_name: None ,
					sid      : None ,
					cid      : None ,
				};

				Err( ThesRemoteErr::ConnectionClosed{ ctx } )
			}
		}
	}



	// actually send the error accross the wire. This is for when errors happen on receiving
	// messages (eg. Deserialization errors).
	//
	// sid null is important so the remote knows that this is an error.
	//
	async fn send_err<'a>
	(
		&'a mut self                             ,
		     cid  : ConnID ,
		     err  : &'a ConnectionError          ,

		     // whether the connection should be closed (eg stream corrupted)
		     //
		     close: bool                         ,
	)
	{
		let identity = self.identify();

		if let Some( ref mut out ) = self.outgoing
		{
			trace!( "{}: sending OUT ConnectionError", identity );

			let serialized = serde_cbor::to_vec( err ).expect( "serialize response" );

			// sid null is the marker that this is an error message.
			//
			let msg = WireFormat::create
			(
				ServiceID::null() ,
				cid               ,
				serialized.into() ,
			);

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
	pub fn prep_error( cid: ConnID, err: &ConnectionError ) -> WireFormat
	{
		let serialized = serde_cbor::to_vec( err ).expect( "serialize response" );

		// sid null is the marker that this is an error message.
		//
		WireFormat::create
		(
			ServiceID::null() ,
			cid               ,
			serialized.into() ,
		)
	}


	pub fn identify( &self ) -> String
	{
		match &self.addr
		{
			Some (addr) =>
			{
				let name = match addr.name()
				{
					Some( n ) => format!( ", name: {}", n ),
					None      => "".to_string()            ,
				};

				format!( "Peer (actor_id: {}{})",  addr.id(), name )
			}

			None => "Peer (shutting down)".into(),
		}
	}



	// Doesn't take &self, because it needs to be called from spawned tasks
	// which have to be 'static.
	//
	#[ doc( hidden ) ]
	//
	pub fn err_ctx
	(
		addr   : &Addr<Self>                  ,
		sid    : impl Into<Option<ServiceID>> ,
		cid    : impl Into<Option<ConnID>>    ,
		context: impl Into<Option<String>>    ,
	)

		-> ErrorContext
	{
		ErrorContext
		{
			peer_id  : addr.id().into()         ,
			peer_name: addr.name().map( |n| n ) ,
			context  : context.into()           ,
			sid      : sid.into()               ,
			cid      : cid.into()               ,
		}
	}
}



// Put an outgoing multiservice message on the wire.
// TODO: why do we not return the error?
//
impl Handler<WireFormat> for Peer
{
	fn handle( &mut self, msg: WireFormat ) -> Return<'_, ()>
	{
		Box::pin( async move
		{
			trace!( "{}: sending OUT wireformat", self.identify() );

			let _ = self.send_msg( msg ).await;

		})
	}
}



// Pharos, shine!
//
impl Observable<PeerEvent> for Peer
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



impl fmt::Debug for Peer
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "Peer" )
	}
}


impl Drop for Peer
{
	// TODO: only do processing if logging is on.
	//
	fn drop( &mut self )
	{
		let mut id = String::new();

		if let Some( ref addr ) = self.addr
		{
			id = format!( ": {:?}", addr );
		}

		trace!( "Drop peer {}", id );
	}
}
