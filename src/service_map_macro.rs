/// This is a beefy macro which is your main interface to using the remote actors. It's unavoidable to
/// require code in the client application because thespis does not know the types of messages you will
/// create, yet we aim at making the difference between local and remote actors seamless for user code.
///
/// This macro will allow deserializing messages to the correct types, as well as creating recipients for
/// remote actors.
///
/// I have named the parameters for clarity, however it's a macro, and not real named parameters so the
/// order needs to be exact.
///
/// Please open declaration section to see the parameter documentation. The module [remote] has more
/// documentation on using remote actors and there are examples in the `examples/remote` folder to see
/// it all in action. There are many integration tests as well testing each feature of the remote actors
/// in the `tests/remote` folder..
///
/// A unique service id is crated for each service based on the "<namespace>::<service>". It uses
/// the exact strings you provide to the macro. Server and client need to provide the exact same
/// parameters to the macro in order to be able to communicate, eg. if you refer to the service types
/// as some path (eg. `module::Type`), both server and client need to do so.
///
/// Types created by this macro, for the following invocation:
///
/// ```ignore
///
/// service_map!
/// (
///    namespace: myns;
///
///    services:
///
///       ServiceA,
///       ServiceB,
/// );
///
/// mod myns
/// {
///    // sid will be different for ServiceA in another service map with another namespace than myns
///    //
///    impl Service for ServiceA {...} // self being myns
///    impl Service for ServiceB {...}
///
///    pub struct Services {}
///
///    impl Namespace for Services { const NAMESPACE: &'static str = "myns"; }
///
///    impl Services
///    {
///       /// Creates a recipient to a Service type for a remote actor, which can be used in exactly the
///       /// same way as if the actor was local. This is for the process that wants to use the services
///       /// not the one that provides them. For it to work, they must use the same namespace.
///       //
///       pub fn recipient<S>( peer: Addr<Peer> ) -> impl Address<S> {...}
///
///       ...
///     }
///
///     // Service map is defined in the thespis crate. This exposes the register_handler method so you can
///     // register actors that handle incoming services, and call register_with_peer to tell the
///     // service map to register all services for which it has handlers with a peer.
///     //
///     impl ServiceMap for Services {...}
///
///     // Some types to make the impl Address<S> in Services::recipient above.
/// }
/// ```
///
//
#[ macro_export ]
//
macro_rules! service_map
{

(
	/// namespace unique to this servicemap. It allows you to use your services with several service_maps
	/// and it also gets used in the unique ID generation of services, so different processes can expose
	/// services based on the same type which shall be uniquely identifiable.
	///
	/// A process wanting to send messages needs to create the service map with the same namespace as the
	/// receiving process.
	//
	namespace: $ns: ident;

	/// Which WireFormat to use.
	//
	wire_format: $wf: path;

	/// Comma separated list of Services you want to include. They must be in scope.
	//
	services: $($services: path),+ $(,)? $(;)?
) =>

{

pub mod $ns
{

use
{
	// It's important the comma be inside the parenthesis, because the list might be empty, in which
	// we should not have a leading comma before the next item, but if the comma is after the closing
	// parenthesis, it will not output a trailing comma, which will be needed to separate from the next item.
	//
	super :: { $( $services, )+ Peer                                                                          } ,
	$crate:: { *, peer::request_error::RequestError                                                           } ,
	std   :: { pin::Pin, collections::HashMap, fmt, any::Any, sync::{ Arc, Once }, ops::Deref, future::Future } ,

	$crate::external_deps::
	{
		once_cell       :: { sync::Lazy                                   } ,
		futures         :: { future::FutureExt, task::{ Context, Poll }   } ,
		thespis         :: { *                                            } ,
		thespis_impl    :: { Addr, Receiver, ThesErr, ThesRes             } ,
		serde_cbor      :: { self, from_slice as des                      } ,
		serde           :: { Serialize, Deserialize, de::DeserializeOwned } ,
		log             :: { error                                        } ,
		parking_lot     :: { Mutex                                        } ,
		paste,
	},
};



/// A [Message] that can be received from remote code. Mainly defines that this [Message] type has
/// a unique id which allows distinguishing it from other services. It is namespaced, so that different
/// components/processes can expose services to the network which will accept the same [Message] type,
/// yet give them a unique identifier.
///
pub trait Service

	// TODO: From trying with Deserialize<'de>, and reading https://serde.rs/lifetimes.html#trait-bounds
	// I think we need DeserializeOwned here. I'm not a 100% sure though, so hence this comment if
	// at later stage we can have certainty here, then replace this comment with a proof of why
	// we need which one.
	//
	where  Self                    : Message + Serialize + DeserializeOwned,
         <Self as Message>::Return:           Serialize + DeserializeOwned,
{
	/// The unique service id. It needs to be static. You can create runtime static data with
	/// lazy_static or OnceCell. That way it will only have to be generated once per service per
	/// process run. Even better is to be able to generate it from const code so it has no runtime
	/// overhead.
	///
	/// For a given Service and Namespace, the output should always be the same, even accross processes
	/// compiled with different versions of rustc. Ideally the algorithm is also clearly described so
	/// programs written in other languages can also communicate with your services.
	//
	fn sid() -> ServiceID where Self: Sized;
}





$(

	impl Service for $services
	{
		fn sid() -> ServiceID
		{
			static INSTANCE : Lazy< ServiceID > = Lazy::new( ||

				ServiceID::from_seed( stringify!( $ns::$services ).as_bytes() )
			);

			*INSTANCE
		}
	}

)+


/// The actual service map.
/// Use it to get a recipient to a remote service.
//
pub struct Services
{
	// The addresses to the actors that handle incoming messages.
	//
	handlers: HashMap< ServiceID, Mutex<Box<dyn Any + Send>> >,
}



/// Will print something like:
///
/// ```ignore
/// remotes::Services
/// {
///    Add  - sid: 0xbcc09d3812378e171ad366d75f687757 - handler: id(0), name(actor_name)
///    Show - sid: 0xbcc09d3812378e17e1a1e89b512c025a - handler: id(0), name(actor_name)
/// }
/// ```
//
impl fmt::Debug for Services
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		let mut width: usize = 0;

		$(
			width = std::cmp::max( width, stringify!( $services ).len() );
		)+

		write!( f, "{}::Services\n{{\n", stringify!( $ns ) )?;


		$(
			let sid = <$services as Service>::sid();

			write!
			(
				f,

				"\t{:width$} - sid: 0x{:02x} - handler: ",

				stringify!( $services ),
				sid,

				width = width
			)?;

			if let Some(h) = self.handlers.get( &sid )
			{
				let h = h.lock();

				// This expect shouldn't ever fail. We manually make the receiver in this file.
				//
				let handler: &Receiver<$services> = h.downcast_ref().expect( "downcast receiver in Debug for Services" );

				match handler.name()
				{
					Some(n) => write!( f, "id({}), name({})", &handler.id(), &n )?,
					None    => write!( f, "id({})", &handler.id() )?,
				};
			}

			else
			{
				write!( f, "none" )?;
			}

			write!( f, "\n" )?;
		)+

		write!( f, "}}" )
	}
}



/// This downcasts in order to clone the handlers
//
impl Clone for Services
{
	fn clone( &self ) -> Self
	{
		#[ allow(clippy::mutable_key_type) ] // false positive.
		//
		let mut handlers: HashMap< ServiceID, Mutex<Box<dyn Any + Send>> > = HashMap::new();

		for (k, v) in &self.handlers
		{
			match k
			{
				$(
					_ if *k == <$services as Service>::sid() =>
					{
						// This should never fail, we make this type in this file.
						//
						let v = v.lock();
						let h: &Receiver<$services> = v.downcast_ref().expect( "downcast receiver in Clone" );

						handlers.insert( *k, Mutex::new( Box::new( h.clone() ) ) );
					},
				)+


				// every sid in our handlers map should also be a valid service in this service map,
				// so this should never happen
				//
				_ => { unreachable!() },
			}

		}

		Self { handlers }
	}
}



impl Services
{
	/// Create a new service map
	//
	pub fn new() -> Self
	{
		$(
			paste::expr!
			{
				static [< __ONCE__ $services >]: Once = Once::new();

				[< __ONCE__ $services >].call_once( ||
				{
					ServiceID::register_service( $services::sid(), concat!( stringify!($ns) , "::", stringify!($services) ) );
				});
			}
		)+

		Self{ handlers: HashMap::new() }
	}


	/// Register a handler for a given service type
	/// Calling this method twice for the same type will override the first handler.
	//
	pub fn register_handler<S>( &mut self, handler: BoxAddress<S, ThesErr> )

		where  S                    : Service,
		      <S as Message>::Return: Serialize + DeserializeOwned,
	{
		self.handlers.insert( <S as Service>::sid(), Mutex::new(Box::new( Receiver::new(handler) )) );
	}


	// Helper function for call_service below.
	// The receiver passed in here keeps a mutex locked. This method should never be async, nor await anything.
	//
	fn call_service_gen<S>
	(
		    msg      :  $wf                   ,
		    receiver : &Box< dyn Any + Send > ,
		mut ctx      :  PeerErrCtx            ,

	) -> Result< Pin<Box< dyn Future< Output=Result<Response<$wf>, PeerErr> > + Send >>, PeerErr >

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send + ,

	{
		let sid = <S as Service>::sid();

		// Deserialize the message.
		//
		let message: S = match des( &msg.mesg() )
		{
			Ok (x) => x,
			Err(_) => return Err( PeerErr::Deserialize{ ctx } )
		};


		// Downcast the receiver, should never fail as we make it in this file.
		//
		let backup: &Receiver<S> = receiver.downcast_ref()

			.expect( "downcast receiver in call_service_gen" );


		let mut rec = backup.clone_box() ;
		let     cid = msg.cid()      ;

		Ok( async move
		{
			// Call the service and wait for the response
			//
			let response = match rec.call( message ).await
			{
				Ok(x) => x,

				Err(_) =>
				{
					// unwrap: we pass it in from below in this file, it has a context guaranteed.
					//
					ctx.context.as_mut().map( |c| c.push_str( " - Process call for local Actor" ) );

					return Err( PeerErr::HandlerDead{ ctx } );
				}
			};


			// Create a $wf response.
			// The sid must be full to differentiate a response from a request. If the request
			// has timed out, the remote peer will no longer have the cid in their list of open requests,
			// so they would not know this was a response otherwise.
			//
			let mut wf = <$wf>::with_capacity( ::std::mem::size_of::<S>() );
			wf.set_sid( ServiceID::full() );
			wf.set_cid( cid               );

			// serialize the response
			//
			serde_cbor::to_writer( &mut wf, &response ).map_err( |_|
			{
				ctx.context.as_mut().map( |c| c.push_str( " - Response to remote call" ) );

				PeerErr::Serialize{ ctx }

			})?;


			Ok( Response::CallResponse( CallResponse::new(wf) ))

		}.boxed() )
	}
}


impl ServiceMap<$wf> for Services
{
	// We need to make a Vec here because the hashmap.keys() doesn't have a static lifetime.
	//
	fn services( &self ) -> Vec<ServiceID>
	{
		let mut s: Vec<ServiceID> = Vec::with_capacity( self.handlers.len() );

		for sid in self.handlers.keys()
		{
			s.push( *sid );
		}

		s
	}



	/// Will match the type of the service id to deserialize the message and send it to the handling actor.
	///
	/// This can return the following errors:
	/// - PeerErr::UnknownService
	/// - PeerErr::Deserialize
	//
	fn send_service( &self, msg: $wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<$wf>, PeerErr> > + Send >>, PeerErr >

	{
		let sid = msg.sid();
		let ctx = ctx.context( "Services::send_service".to_string() );

		// This sid should be in our map.
		//
		let receiver = self.handlers.get( &sid )

			.ok_or_else( || PeerErr::NoHandler{ ctx: ctx.clone() } )?
			.lock()
		;


		// Map the sid to a type S.
		//
		match sid
		{
			$(
				_ if sid == <$services as Service>::sid() =>
				{
					// This should always succeed, receiver is made in this very file.
					//
					let rec: &Receiver<$services> = receiver.downcast_ref()

						.expect( "downcast receiver in send_service" );


					// Deserialize.
					//
					let message: $services = match des( &msg.mesg() )
					{
						Ok (x) => x,
						Err(_) => return Err( PeerErr::Deserialize{ ctx } ),
					};


					// We need to clone the receiver so it can be inside the future as &mut self.
					//
					let mut rec = rec.clone_box();

					Ok( async move
					{
						match rec.send( message ).await
						{
							Ok (_) => Ok ( Response::Nothing                 ),
							Err(_) => Err( PeerErr::HandlerDead{ ctx } ),
						}

					}.boxed() )
				},
			)+

			_ =>
			{
				Err( PeerErr::NoHandler{ ctx } )
			}
		}
	}



	/// Will match the type of the service id to deserialize the message and call the handling actor.
	/// It returns a future that actually calls the handling actor. This futures is spawned by Peer.
	///
	/// This can return the following errors:
	/// - PeerErr::UnknownService
	/// - PeerErr::Deserialize
	/// - PeerErr::ThesErr -> Spawn error
	//
	fn call_service
	(
		&self              ,
		msg   : $wf        ,
		ctx   : PeerErrCtx ,

	) -> Result< Pin<Box< dyn Future< Output=Result<Response<$wf>, PeerErr> > + Send >>, PeerErr >
	{
		let sid = msg.sid();
		let ctx = ctx.context( "Services::call_service".to_string() );

		let receiver = match self.handlers.get( &sid )
		{
			// FIXME: don't block the thread.
			// Not easy to solve. It doesn't cross await points and it shouldn't lock for very long.
			//
			Some(x) => x.lock(),

			None =>
			{
				return Err( PeerErr::NoHandler{ ctx } )
			}
		};

		match sid
		{
			$(
				_ if sid == <$services as Service>::sid() =>
				{
					Self::call_service_gen::<$services>( msg, &*receiver, ctx )
				}
			)+


			_ => return Err( PeerErr::UnknownService{ ctx } )
		}
	}
}



/// Concrete type for creating recipients for remote Services in this thespis::ServiceMap.
/// Note that this holds the Peers address, so the peer mailbox will only get dropped when you
/// drop this, even if the connection get's closed by the remote.
///
/// If you use this, be sure to check the result that is returned which will err if the connection
/// is closed.
//
#[ derive( Clone, Debug ) ]
//
pub struct RemoteAddr
{
	// FIXME: do not rely on Addr, we should be generic over Address, but not
	//       choose an implementation. This is a complicated one. While this is in the public
	//       API, so it would be good, having a trait object that is Address<$wf> + Address<Call>
	//       and is still cloneable is complicated.
	//
	//       It could be done by unifying both message types (eg. an enum), but then what is the return
	//       type of this message. It would have to be an enum as well, and every caller would have to
	//       match on it. For now we will keep our dependency on Peer and Addr.
	//
	peer: Addr<Peer<$wf>>
}


impl RemoteAddr
{
	pub fn new( peer: Addr<Peer<$wf>> ) -> Self
	{
		Self { peer }
	}


	/// Take the raw message and turn it into a WireFormat
	//
	fn build_wf<S>( msg: S, cid: ConnID ) -> Result< $wf, PeerErr >

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let sid = <S as Service>::sid();

		let mut wf = <$wf>::with_capacity( ::std::mem::size_of::<S>() );
		wf.set_sid( sid );
		wf.set_cid( cid );

		// serialize the response
		//
		serde_cbor::to_writer( &mut wf, &msg ).map_err( |_|
		{
			let mut ctx = PeerErrCtx::default();
			ctx.context = "Outgoing request".to_string().into();
			ctx.sid     = sid.into();

			PeerErr::Serialize{ ctx }

		})?;

		Ok( wf )
	}


	/// Take the raw message and turn it into a Call
	//
	fn build_call<S>( msg: S ) -> Result< Call<$wf>, PeerErr >

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let sid = <S as Service>::sid();

		let mut wf = <$wf>::with_capacity( ::std::mem::size_of::<S>() );
		wf.set_sid( sid );

		// serialize the response
		//
		serde_cbor::to_writer( &mut wf, &msg ).map_err( |_|
		{
			let mut ctx = PeerErrCtx::default();
			ctx.context = "Outgoing request".to_string().into();
			ctx.sid     = sid.into();

			PeerErr::Serialize{ ctx }

		})?;

		Ok( Call::new( wf ) )
	}
}



impl<S> Address<S> for RemoteAddr

	where  S                    : Service + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,

{
	/// Call a remote actor.
	///
	/// ### potential errors
	///
	/// 1. serialization of the outgoing message
	/// 2.
	//
	fn call( &mut self, msg: S ) -> Return<Result< <S as Message>::Return, PeerErr >> { async move
	{
		// Serialization can fail
		//
		let call = Self::build_call( msg )?;

		// Can fail if the peer is down already.
		//
		let rx = self.peer.call( call ).await

			// The peer panicked.
			//
			.map_err( |_|
			{
				let ctx = Peer::err_ctx( &self.peer, <S as Service>::sid(), None, "Call remote service".to_string() );

				PeerErr::PeerGone{ ctx }

			})?

			// The actual sending out over the network can fail.
			//
			.map_err( |_|
			{
				let ctx = Peer::err_ctx( &self.peer, <S as Service>::sid(), None, "Call remote service".to_string() );

				PeerErr::ConnectionClosed{ ctx }

			})?;


		// Channel can be canceled
		//
		let re = rx.await

			.map_err( |_|
			{
				let ctx = PeerErrCtx
				{
					context  : Some( "Peer stopped before receiving response from remote call".to_string() ) ,
					peer_id  : self.peer.id().into()                                                         ,
					peer_name: self.peer.name()                                                              ,
					sid      : <S as Service>::sid().into()                                                  ,
					cid      : None                                                                          ,
				};

				PeerErr::ConnectionClosed{ ctx }

			})?;


		// A response came back from the other side.
		//
		match re
		{
			Ok ( resp ) =>
			{
				// Deserialize the payload and return it to the caller.
				//
				Ok( des( &resp.mesg() )

					.map_err( |_|
					{
						let ctx = PeerErrCtx
						{
							context  : Some( "Response to call from remote actor".to_string() ) ,
							peer_id  : self.peer.id().into()                                    ,
							peer_name: self.peer.name()                                         ,
							sid      : <S as Service>::sid().into()                             ,
							cid      : resp.cid().into()                                        ,
						};

						PeerErr::Deserialize{ ctx }

					})?
				)
			},

			// The remote returned an error.
			//
			Err( err ) =>
			{
				let mut ctx = PeerErrCtx
				{
					context  : Some( "Remote could not process our message".to_string() ) ,
					peer_id  : self.peer.id().into()                                      ,
					peer_name: self.peer.name()                                           ,
					sid      : <S as Service>::sid().into()                               ,
					cid      : None                                                       ,
				};

				match err
				{
					// This is a special case, since it get's returned from the channel it needed to be a
					// ConnectionError, but it doesn't actually come from the remote, so translate into
					// it's own error variant rather than PeerErr::Remote.
					//
					// It can however come from a relay. We don't allow the user here to distinguish whether
					// this peer timed out or a relay.
					//
					// TODO: Document this as users might be confused when they raise their timeout and it still
					// times out. It will be logged as a remote error by the code in peer/incoming.rs
					//
					ConnectionError::Timeout(_) =>
					{
						ctx.context = Some( "Time out waiting for response to outgoing call".to_string() );

						Err( PeerErr::Timeout{ ctx } )
					}

					_ =>
					{
						Err( PeerErr::Remote{ err, ctx } )
					}
				}
			},
		}

	}.boxed() }


	/// Obtain a clone of this recipient as a trait object.
	//
	fn clone_box( &self ) -> BoxAddress<S, PeerErr>
	{
		Box::new( Self { peer: self.peer.clone() } )
	}
}




impl<S> Sink<S> for RemoteAddr

	where  S                    : Service + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,

{
	type Error = PeerErr;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Sink::<$wf>::poll_ready( Pin::new( &mut self.peer ), cx )

			.map_err( |source|
			{
				let ctx = Peer::err_ctx( &self.peer, <S as Service>::sid(), None, "Send on RemoteAddr".to_string() );

				PeerErr::ThesErr{ ctx, source }
			})
	}


	fn start_send( mut self: Pin<&mut Self>, msg: S ) -> Result<(), Self::Error>
	{
		Sink::<$wf>::start_send( Pin::new( &mut self.peer ), Self::build_wf( msg, ConnID::null() )? )

			.map_err( |source|
			{
				let ctx = Peer::err_ctx( &self.peer, <S as Service>::sid(), None, "Send on RemoteAddr".to_string() );

				PeerErr::ThesErr{ ctx, source }
			})
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Sink::<$wf>::poll_flush( Pin::new( &mut self.peer ), cx )

			.map_err( |source|
			{
				let ctx = Peer::err_ctx( &self.peer, <S as Service>::sid(), None, "Send on RemoteAddr".to_string() );

				PeerErr::ThesErr{ ctx, source }
			})
	}


	/// Will only close when dropped, this method can never return ready
	//
	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Poll::Ready(Ok(()))
	}
}


impl Identify for RemoteAddr
{
	/// Unique id of the peer this sends over
	//
	fn id( &self ) -> usize
	{
		self.peer.id()
	}

	/// Unique id of the peer this sends over
	//
	fn name( &self ) -> Option<Arc<str>>
	{
		self.peer.name()
	}
}

}}} // End of macro
