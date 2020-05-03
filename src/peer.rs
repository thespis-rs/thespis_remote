//! The peer module holds everything that deals with managing a remote connection over which
//! actor messages can be sent and received.
//
use crate :: { import::*, * };


    mod backpressure      ;
    mod call              ;
    mod call_response     ;
    mod close_connection  ;
    mod connection_error  ;
    mod incoming          ;
    mod peer_err          ;
    mod peer_event        ;
pub mod request_error     ;
    mod response          ;
    mod timeout           ;

pub use backpressure      :: { BackPressure        } ;
pub use call              :: { Call                } ;
pub use call_response     :: { CallResponse        } ;
pub use close_connection  :: { CloseConnection     } ;
pub use connection_error  :: { ConnectionError     } ;
    use incoming          :: { Incoming            } ;
pub use peer_err          :: { PeerErr, PeerErrCtx } ;
pub use peer_event        :: { PeerEvent           } ;
    use request_error     :: { RequestError        } ;
pub use response          :: { Response            } ;
    use timeout           :: { Timeout             } ;


// Reduce trait bound boilerplate, since we have to repeat them all over
//
/// Trait bounds for the stream of incoming messages
//
pub trait BoundsIn : 'static + Stream< Item = Result<WireFormat, WireErr> > + Unpin + Send {}

/// Trait bounds for the Sink of outgoing messages.
//
pub trait BoundsOut: 'static + Sink<WireFormat, Error=WireErr > + Unpin + Send {}


impl<T> BoundsIn for T

	where T : 'static + Stream< Item = Result<WireFormat, WireErr> > + Unpin + Send
{}

impl<T> BoundsOut for T

	where T : 'static + Sink<WireFormat, Error=WireErr > + Unpin + Send
{}



/// Represents a connection to another process over which you can send/receive actor messages.
///
/// ### Defining which services to expose.
///
/// Peer can make use of any type that implements [ServiceMap]. The ServiceMap knows which services
/// it provides and how to deliver messages to the handling actors. You could implement ServiceMap
/// on your own types, but thespis_remote provides you with 2 implementations out of the box. One
/// for delivery to actors in the local process (requires the macro `service_map!`) and one for
/// relaying messages to other processes.
///
/// The general workflow requires you to specify to the ServiceMap which actors/connections handle
/// certain services, pass it to the peer in `register_services` and the lauch the mailbox for the
/// peer.
///
/// Runtime modification is provided. You can tell the ServiceMap to start delivering to another
/// actor/connection, and you can tell the peer to start/stop exposing a certain service. Once
/// the mailbox for the peer has been started, you can only communicate to it by means of messages,
/// so the messages `AddServices` and `RemoveServices` can be used to convey runtime instructions.
///
/// In principle you setup the peer with at least one ServiceMap before starting it, that way it
/// is fully operational before it receives the first incoming message. `service_map!` let's you
/// set a `dyn Address<S>` as handler, whereas `RelayMap` can accept both an address that receives
/// both `WireFormat` (for Sends) and `peer::Call` for calls, but also a closure that let's you
/// provide such address on a per message basis, which allows you to implement load balancing
/// to several backends providing the same service. Neither of these maps allow you to "unset"
/// a handler, only replace it with a new one. The point is a difference in error messages. It
/// is part of the contract of a ServiceMap that it knows how to deliver messages of all types
/// for which it advertised earlier. Thus if no handler can be found, the remote will receive
/// a `ConnectionError::InternalServer`. When a service isn't present for the peer however
/// (such as after removing it with `RemoveServices`) the remote will receive a
/// `ConnectionError::UnknownService`.
///
/// You can check the documentation of both [`RelayMap`] and [`service_map!`] for more information
/// on their usage.
///
/// You can supply several service maps with different services to Peer. They will only advertise
/// services for which you have actually set handlers. When you later want to add services with
/// `AddServices`, you can pass in the same service map if you want, as long as you have added
/// handlers for all the services you wish to add. When seeding before starting the peer, only
/// one service map may claim to provide a given service. Peer only delivers the message to exactly
/// one handler.
///
/// ### Sending messages to remote processes.
///
/// As far as the Peer type is concerned sending actor messages to a remote is relatively simple.
/// Once you have serialized a message as `WireFormat`, sending that directly to `Peer` will be considered
/// a Send to a remote actor and it will just be sent out. For a Call, there is the Call message type,
/// which will resolve to a channel you can await in order to get your response from the remote process.
/// The `service_map!` macro provides a `RemoteAddress` type which acts much the same as a local actor address
/// and will accept messages of all services that are defined in the service map.
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
	outgoing: Option< Box<dyn BoundsOut> >,

	/// This is needed so that the loop listening to the incoming stream can send messages to this actor.
	/// The loop runs in parallel of the rest of the actor, yet processing incoming messages need mutable
	/// access to our state, so we have to pass through a message, or we need to put everything in Rc<RefCell>>.
	/// For now, passing messages seems the cleaner solution.
	///
	/// It also allows us to hand out our address to things that have to respond to the remote on our connection.
	//
	addr: Option< Addr<Self> >,

	// The ID of this actor. Since the addr isn't always there (eg. after we start closing the connection),
	// it's a royal PITA not to have these directly available. Keeping them after the addr is gone also means
	// better error messages.
	//
	id  : usize,
	name: Option<Arc<str>>,

	/// Information required to process incoming messages. The first element is a boxed Receiver, and the second is
	/// the service map that takes care of this service type.
	//
	services: HashMap< ServiceID, Arc<dyn ServiceMap> >,

	/// We use oneshot channels to give clients a future that will resolve to their response.
	//
	responses: HashMap< ConnID, oneshot::Sender<Result<WireFormat, ConnectionError>> >,

	/// The pharos allows us to have observers.
	//
	pharos: Pharos<PeerEvent>,

	// How long to wait for responses to outgoing requests before timing out.
	//
	timeout: Duration,

	// How long to wait for responses to outgoing requests before timing out.
	//
	backpressure: Option<Arc< BackPressure >>,

	// All spawned requests will be collected here.
	//
	nursery_stream: Option<JoinHandle<Result<Response, PeerErr>>>,

	// All spawned requests will be collected here.
	//
	nursery: Nursery<Arc< dyn SpawnHandle<Result<Response, PeerErr>> + Send + Sync + 'static >, Result<Response, PeerErr>>,

	// Set by close connection. We no longer want to process any messages after this.
	//
	closed: bool,
}



impl Peer
{
	/// Create a new peer to represent a connection to some remote.
	/// `addr` is the actor address for this actor.
	//
	pub fn new
	(
		addr     : Addr<Self>                                                                ,
		incoming : impl BoundsIn                                                             ,
		outgoing : impl BoundsOut                                                            ,
		exec     : impl SpawnHandle<Result<Response, PeerErr>> + Send + Sync + 'static ,
		bp       : Option<Arc<BackPressure>>                                                 ,
	)

		-> Result< Self, PeerErr >

	{
		trace!( "{}: Create peer", &addr );


		let exec: Arc< dyn SpawnHandle<Result<Response, PeerErr>> + Send + Sync + 'static > = Arc::new(exec);
		let (nursery, nursery_stream) = Nursery::new( exec.clone() );


		let nursery_handle = exec.spawn_handle( Self::listen_request_results( nursery_stream, addr.clone() ) )

			.map_err( |_| -> PeerErr
			{
				let mut ctx = PeerErrCtx::default();
				ctx.context = "Request results stream for peer".to_string().into();
				PeerErr::Spawn{ ctx }
			})?
		;


		nursery.nurse( Self::listen_incoming( incoming, addr.clone(), bp.clone() ) )

			.map_err( |_| -> PeerErr
			{
				let mut ctx = PeerErrCtx::default();
				ctx.context = "Incoming stream for peer".to_string().into();
				PeerErr::Spawn{ ctx }
			})?
		;


		Ok( Self
		{
			id             : addr.id()                  ,
			name           : addr.name()                ,
			outgoing       : Some( Box::new(outgoing) ) ,
			addr           : Some( addr )               ,
			responses      : HashMap::new()             ,
			services       : HashMap::new()             ,
			pharos         : Pharos::default()          ,
			timeout        : Duration::from_secs(60)    ,
			backpressure   : bp                         ,
			closed         : false                      ,
			nursery_stream : Some( nursery_handle )     ,
			nursery                                     ,
		})
	}



	/// The task that will listen to results returned by the spawned tasks that process requests
	/// as well as some other tasks that need to be confined to the lifetime of the Peer. These
	/// include tasks spawned for timeouts, the task listening to incoming requests, ...
	///
	/// For request processing this either forwards responses to the peer for sending out, or errors
	/// that need to be reported to the remote. In general it is considered that the errors returned directly
	/// by the methods on ServiceMap contain relevant information for the Remote, but errors coming from
	/// within the spawned tasks are just internal server errors. In any case RequestError will do
	/// the right thing based on the error type, so this just forwards it to the handler for
	/// RequestError.
	///
	/// It just returns a result so that it can be spawned on the same executor. It's infallible.
	//
	async fn listen_request_results
	(
		mut stream: NurseryStream<Result<Response, PeerErr>> ,
		mut addr  : Addr<Peer>                               ,
	)
		-> Result<Response, PeerErr>

	{
		while let Some(result) = stream.next().await
		{
			// The result of addr.send
			//
			let res = match result
			{
				Ok(resp) => match resp
				{
					Response::Nothing         => Ok(())               ,
					Response::WireFormat  (x) => addr.send( x ).await ,
					Response::CallResponse(x) => addr.send( x ).await ,
				}

				Err(err) => addr.send( RequestError::from( err.clone() ) ).await
			};

			// As we hold an address, the only way the mailbox can already be shut
			// is if the peer panics, or the mailbox get's dropped. Since the mailbox
			// owns the Peer, if it's dropped, this task doesn't exist anymore.
			// Should never happen.
			//
			debug_assert!( res.is_ok() );
		}

		Ok(Response::Nothing)
	}



	/// The task that will listen to incoming messages on the network connection and send them to our
	/// the peer's address.
	//
	async fn listen_incoming
	(
		mut incoming: impl BoundsIn             ,
		mut addr    : Addr<Peer>                ,
		    bp      : Option<Arc<BackPressure>> ,
	)
		-> Result<Response, PeerErr>

	{
		// This can fail if:
		//
		// - the receiver is dropped. The receiver is our mailbox, so it should never be dropped
		//   as long as we have an address to it and this task would be dropped with it.
		//
		while let Some(msg) = incoming.next().await
		{
			if let Some( ref bp ) = bp
			{
				trace!( "check for backpressure" );

				bp.wait().await;

				trace!( "backpressure allows progress now." );
			}

			trace!( "{}: incoming message.", &addr );

			if addr.send( Incoming{ msg } ).await.is_err()
			{
				error!( "{} has panicked or it's inbox has been dropped.", Peer::identify_addr( &addr ) );
			}
		}

		trace!( "{}:  incoming stream end, closing out.", &addr );

		// The connection was closed by remote, tell peer to clean up.
		//
		let res = addr.send( CloseConnection{ remote: true, reason: "Connection closed by remote.".to_string() } ).await;

		// As we hold an address, the only way the mailbox can already be shut
		// is if the peer panics, or the mailbox get's dropped. Since the mailbox
		// owns the Peer, if it's dropped, this task doesn't exist anymore.
		// Should never happen.
		//
		debug_assert!( res.is_ok() );

		Ok(Response::Nothing)
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
		addr        : Addr<Self>                                                                ,
		socket      : impl FutAsyncRead + FutAsyncWrite + Unpin + Send + 'static                ,
		max_size    : usize                                                                     ,
		exec        : impl SpawnHandle<Result<Response, PeerErr>> + Send + Sync + 'static ,
		bp          : Option<Arc<BackPressure>>                                           ,
	)

		-> Result< Self, PeerErr >

	{
		let codec = ThesCodec::new(max_size);

		let (sink, stream) = FutFramed::new( socket, codec ).split();

		Peer::new( addr, stream, sink, Arc::new(exec), bp )
	}



	#[ cfg( feature = "tokio_codec" ) ]
	//
	/// Create a Peer directly from a tokio asynchronous stream. This is a convenience wrapper around Peer::new so
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
	pub fn from_tokio_async_read
	(
		addr        : Addr<Self>                                                          ,
		socket      : impl TokioAsyncR + TokioAsyncW + Unpin + Send + 'static             ,
		max_size    : usize                                                               ,
		exec        : impl SpawnHandle<Result<Response, PeerErr>> + Send + Sync + 'static ,
		bp          : Option<Arc<BackPressure>>                                           ,
	)

		-> Result< Self, PeerErr >

	{
		let codec = ThesCodec::new(max_size);

		let (sink, stream) = TokioFramed::new( socket, codec ).split();

		Peer::new( addr, stream, sink, exec, bp )
	}


	/// Set the timeout for outgoing calls. This defaults to 60 seconds if not set by this method.
	/// Having a timeout allows your code to detect if a remote is not reactive and prevents a memory
	/// leak in Peer where information regarding the request would be kept indefinitely otherwise.
	//
	pub fn set_timeout( &mut self, delay: Duration )
	{
		self.timeout = delay;
	}


	/// Register a service map as the handler for service ids that come in over the network. Normally you should
	/// not call this directly, but use [´thespis_remote::ServiceMap::register_with_peer´].
	///
	/// Each service map and each service should be registered only once, including relayed services. Trying to
	/// register them twice will panic in debug mode.
	//
	pub fn register_services( &mut self, sm: Arc< dyn ServiceMap> )
	{
		for sid in sm.services().into_iter()
		{
			trace!( "{}: Register Service: {:?}", self.identify(), &sid );

			debug_assert!
			(
				!self.services.contains_key( &sid ),
				"{}: Register Service: Can't register same service twice. sid: {}", self.identify(), &sid ,
			);

			self.services.insert( sid, sm.clone() );
		}
	}


	// actually send the message accross the wire
	//
	async fn send_msg( &mut self, msg: WireFormat ) -> Result<(), PeerErr>
	{
		trace!( "{}: sending OUT WireFormat", self.identify() );

		match &mut self.outgoing
		{
			Some( out ) =>
			{
				let sid = msg.service();
				let cid = msg.conn_id();

				out.send( msg ).await

					.map_err( |source|
					{
						let ctx = self.ctx( sid, cid, "Sending out WireFormat" );
						PeerErr::WireFormat{ ctx, source }
					})
			}

			None =>
			{
				let ctx = PeerErrCtx::default().context( "register_relayed_services".to_string() );

				Err( PeerErr::ConnectionClosed{ ctx } )
			}
		}
	}



	// Actually send the error accross the wire. This is for when errors happen on receiving
	// messages (eg. Deserialization errors).
	//
	// sid null is important so the remote knows that this is an error.
	//
	// This is currently only called from RequestError.
	//
	async fn send_err
	(
		&mut self               ,

		cid  : ConnID           ,
		err  : &ConnectionError ,
		close: bool             , // whether the connection should be closed (eg stream corrupted)
	)
	{
		trace!( "{}: sending OUT ConnectionError", self.identify() );

		// If self.outgoing is None, we have already closed.
		//
		let out = match self.outgoing
		{
			Some( ref mut out ) => out,
			None                => return,
		};

		// There is no documentation on serde_cbor or serde_derive about why this would return an error.
		// As far as I can tell it shouldn't ever fail.
		//
		let serialized = serde_cbor::to_vec( err ).expect( "serialize ConnectionError" );

		// sid null is the marker that this is an error message.
		//
		let msg = WireFormat::create( ServiceID::null(), cid, serialized.into() );

		// We are already trying to report an error. If we can't send, just give up.
		//
		let _ = out.send( msg ).await;

		if close
		{
			let close_conn = CloseConnection{ remote: false, reason: format!( "{:?}", err ) };

			Handler::<CloseConnection>::handle( self, close_conn ).await
		}
	}



	// serialize a ConnectionError to be sent across the wire. Public because used in
	// sevice_map macro.
	//
	#[ doc( hidden ) ]
	//
	pub fn prep_error( cid: ConnID, err: &ConnectionError ) -> WireFormat
	{
		// There is no documentation on serde_cbor or serde_derive about why this would return an error.
		// As far as I can tell it shouldn't ever fail.
		//
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
		match self.name
		{
			None           => format!( "Peer: id: {}"          , self.id       ),
			Some(ref name) => format!( "Peer: id: {}, name: {}", self.id, name ),
		}
	}


	pub fn identify_addr( addr: &Addr<Peer> ) -> String
	{
		match addr.name()
		{
			None           => format!( "Peer: id: {}"          , addr.id()       ),
			Some(ref name) => format!( "Peer: id: {}, name: {}", addr.id(), name ),
		}
	}



	// generate an error context for convenience.
	//
	fn ctx
	(
		&self                                 ,
		sid    : impl Into<Option<ServiceID>> ,
		cid    : impl Into<Option<ConnID>>    ,
		context: impl AsRef<str>              ,
	)

		-> PeerErrCtx
	{
		PeerErrCtx
		{
			peer_id  : self.id.into()                      ,
			peer_name: self.name.clone()                   ,
			context  : context.as_ref().to_string().into() ,
			sid      : sid.into()                          ,
			cid      : cid.into()                          ,
		}
	}



	// Convenience function for generating the error context.
	//
	// Doesn't take &self, because it get's used by external code which only has an address.
	//
	// This is used in RemoteAddress.
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

		-> PeerErrCtx
	{
		PeerErrCtx
		{
			peer_id  : addr.id().into() ,
			peer_name: addr.name()      ,
			context  : context.into()   ,
			sid      : sid.into()       ,
			cid      : cid.into()       ,
		}
	}
}



// Put an outgoing multiservice message on the wire.
//
impl Handler<WireFormat> for Peer
{
	#[async_fn] fn handle( &mut self, msg: WireFormat ) -> Result<(), PeerErr>
	{
		trace!( "{}: sending OUT wireformat", self.identify() );

		self.send_msg( msg ).await
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
		write!( f, "{}", self.identify() )
	}
}


impl Drop for Peer
{
	fn drop( &mut self )
	{
		trace!( "Drop {}", self.identify() );
	}
}
