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
///    	ServiceA,
///    	ServiceB,
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
	///
	/// TODO: are we sure that path is better than ty or ident? We should test it actually works with paths.
	///       one place where it makes a difference, is in the use statement just below, where ty doesn't
	///       seem to work.
	//
	namespace: $ns: ident;

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
	super :: { $( $services, )+ Peer                                     } ,
	$crate:: { *, peer::request_error::RequestError                      } ,
	std   :: { pin::Pin, collections::HashMap, fmt, any::Any, sync::Once } ,

	$crate::external_deps::
	{
		once_cell       :: { sync::Lazy                                   } ,
		futures         :: { future::FutureExt, task::{ Context, Poll }   } ,
		thespis         :: { *                                            } ,
		thespis_impl    :: { Addr, Receiver, ThesErr, ThesRes             } ,
		serde_cbor      :: { self, from_slice as des                      } ,
		serde           :: { Serialize, Deserialize, de::DeserializeOwned } ,
		log             :: { error                                        } ,
		paste,
	},
};



/// A [Message] that can be received from remote code. Mainly defines that this [Message] type has
/// a unique id which allows distinguishing it from other services. It is namespaced, so that different
/// components/processes can expose services to the network which will accept the same [Message] type,
/// yet give them a unique identifier.
///
pub trait Service

	// TODO: make peace with serde and figure out once and for all when it makes sense
	//       to create a bound to Deserialize<'de> and when to use DeserializeOwned.
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
	fn sid() -> &'static ServiceID where Self: Sized;
}





$(

	impl Service for $services
	{
		fn sid() -> &'static ServiceID
		{
			static INSTANCE : Lazy< ServiceID > = Lazy::new( ||

				ServiceID::from_seed( stringify!( $ns ).as_bytes(), stringify!( $services ).as_bytes() )
			);

			&INSTANCE
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
	handlers: HashMap< &'static ServiceID, Box< dyn Any + Send + Sync > >,
}



/// TODO: Print actor name if it has one
///
/// Will print something like:
///
/// ```ignore
/// remotes::Services
/// {
///    Add  - sid: c5c22cab4f2d334e0000000000000000 - handler (actor_id): 0
///    Show - sid: 0617b1cfb55b99700000000000000000 - handler (actor_id): 0
/// }
/// ```
//
impl fmt::Debug for Services
{
	fn fmt( &self, f: &mut fmt::Formatter ) -> fmt::Result
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

				"\t{:width$} - sid: 0x{:02x} - handler (actor_id): {}\n",

				stringify!( $services ),
				sid,

				if let Some(h) = self.handlers.get( sid )
				{
					// TODO, don't expect?
					//
					let handler: &Receiver<$services> = h.downcast_ref().expect( "downcast receiver in Debug for Services" );
					format!( "{:?}", handler.actor_id() )
				}

				else
				{
					"none".to_string()
				},

				width = width
			)?;
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
		let mut map: HashMap<&'static ServiceID, Box< dyn Any + Send + Sync >> = HashMap::new();


		for (k, v) in &self.handlers
		{
			match k
			{
				$(
					_ if *k == <$services as Service>::sid() =>
					{
						// TODO: don't expect?
						//
						let h: &Receiver<$services> = v.downcast_ref().expect( "downcast receiver in Clone" );

						map.insert( k, Box::new( h.clone() ) );
					},
				)+


				// every sid in our handlers map should also be a valid service in this service map,
				// so this should never happen
				//
				_ => { unreachable!() },
			}

		}

		Self { handlers: map }
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
	pub fn register_handler<S>( &mut self, handler: Receiver<S> )

		where  S                    : Service,
		      <S as Message>::Return: Serialize + DeserializeOwned,
	{
		self.handlers.insert( <S as Service>::sid(), Box::new( handler ) );
	}


	// Helper function for call_service below
	//
	fn call_service_gen<S>
	(
		    msg        :  WireFormat                   ,
		    receiver   : &Box< dyn Any + Send + Sync > ,
		mut peer       :  Addr<Peer>                   ,

	) -> Return<'static, ()>

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let sid = <S as Service>::sid().clone();

		// Deserialize the message.
		//
		let message: S = match des( &msg.mesg() )
		{
			Ok(x) => x,

			Err(_) =>
			{
				let ctx = Peer::err_ctx( &peer, sid.clone(), msg.conn_id(), "Actor message while processing call for local Actor".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::Deserialize{ ctx } ).boxed();
			}
		};


		// Downcast the receiver
		//
		let backup: &Receiver<S> = match receiver.downcast_ref()
		{
			Some(x) => x,

			None =>
			{
				let ctx = Peer::err_ctx( &peer, sid.clone(), msg.conn_id(), "Handler".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::Downcast{ ctx } ).boxed();
			}
		};


		let mut rec  = backup.clone_box() ;
		let     cid  = msg.conn_id()      ;

		async move
		{
			// Call the service and wait for the response
			//
			let response = match rec.call( message ).await
			{
				Ok(x) => x,

				Err(_) =>
				{
					let ctx = Peer::err_ctx( &peer, sid.clone(), cid, "Process call for local Actor".to_string() );

					return Self::handle_err( peer, ThesRemoteErr::HandlerDead{ ctx } ).await;
				}
			};


			// serialize the response
			//
			let ser = serde_cbor::to_vec( &response );

			let serialized = match ser
			{
				Ok(x) => x,

				Err(_) =>
				{
					let ctx = Peer::err_ctx( &peer, sid.clone(), cid, "Response to remote call".to_string() );

					return Self::handle_err( peer, ThesRemoteErr::Serialize{ ctx } ).await;
				}
			};


			// Create a WireFormat response
			//
			let response = WireFormat::create( sid, cid, serialized.into() ) ;


			// Send the response out over the network.
			//
			if peer.send( response ).await.is_err()
			{
				error!( "Peer: {}{:?}, processing incoming call: peer to client is closed before we finished sending a response to a request.", peer.id(), peer.name() );
			}

		}.boxed()
	}


	async fn handle_err( mut peer: Addr<Peer>, err: ThesRemoteErr )
	{
		if peer.send( RequestError::from( err.clone() ) ).await.is_err()
		{
			error!
			(
				"Peer ({}, {:?}): Processing incoming call: peer to client is closed, but processing request errored on: {}.",
				peer.id(),
				peer.name(),
				&err
			);
		}
	}
}



impl ServiceMap for Services
{
	// We need to make a Vec here because the hashmap doesn't have a static lifetime.
	//
	fn services( &self ) -> Vec<&'static ServiceID>
	{
		let mut s: Vec<&'static ServiceID> = Vec::with_capacity( self.handlers.len() );

		for sid in self.handlers.keys()
		{
			s.push( sid );
		}

		s
	}



	/// Will match the type of the service id to deserialize the message and send it to the handling actor.
	///
	/// This can return the following errors:
	/// - ThesRemoteErr::Downcast
	/// - ThesRemoteErr::UnknownService
	/// - ThesRemoteErr::Deserialize
	///
	/// # Panics
	/// For the moment this can panic if the downcast to Receiver fails. It should never happen unless there
	/// is a programmer error, but even then, it should be type checked, so for now I have decided to leave
	/// the expect in there. See if anyone manages to trigger it, we can take it from there.
	///
	//
	fn send_service( &self, msg: WireFormat, peer: Addr<Peer> )

		-> Return<'static, ()>

	{
		let sid = msg.service();

		// This sid should be in our map.
		//
		let receiver = match self.handlers.get( &sid )
		{
			Some(x) => x,

			None =>
			{
				let ctx = Peer::err_ctx( &peer, sid, None, "Processing incoming Send".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::NoHandler{ ctx } ).boxed()
			}
		};


		// Map the sid to a type S.
		//
		match sid
		{
			$(
				_ if sid == *<$services as Service>::sid() =>
				{
					let rec: &Receiver<$services> = match receiver.downcast_ref()
					{
						Some(x) => x,

						None =>
						{
							let ctx = Peer::err_ctx( &peer, sid.clone(), None, "Receiver in send_service".to_string() );

							return Self::handle_err( peer, ThesRemoteErr::Downcast{ ctx } ).boxed()
						}
					};


					let message: $services = match des( &msg.mesg() )
					{
						Ok(x) => x,

						Err(_) =>
						{
							let ctx = Peer::err_ctx( &peer, sid, None, "Actor message in send_service".to_string() );

							return Self::handle_err( peer, ThesRemoteErr::Deserialize{ ctx } ).boxed()
						}
					};


					// We need to clone the receiver so it can be inside the future as &mut self.
					//
					let mut rec = rec.clone_box();

					async move
					{
						if rec.send( message ).await.is_err()
						{
							let ctx = Peer::err_ctx( &peer, sid.clone(), None, "Process Send for local Actor".to_string() );

							return Self::handle_err( peer, ThesRemoteErr::HandlerDead{ ctx } ).await;
						}

					}.boxed()
				},
			)+

			_ =>
			{
				let ctx = Peer::err_ctx( &peer, sid, None, "Processing incoming Send".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::NoHandler{ ctx } ).boxed()
			}
		}
	}



	/// Will match the type of the service id to deserialize the message and call the handling actor.
	///
	/// This can return the following errors:
	/// - ThesRemoteErr::Downcast
	/// - ThesRemoteErr::UnknownService
	/// - ThesRemoteErr::Deserialize
	/// - ThesRemoteErr::ThesErr -> Spawn error
	///
	/// # Panics
	/// For the moment this can panic if the downcast to Receiver fails. It should never happen unless there
	/// is a programmer error, but even then, it should be type checked, so for now I have decided to leave
	/// the expect in there. See if anyone manages to trigger it, we can take it from there.
	///
	//
	fn call_service
	(
		&self               ,
		msg   :  WireFormat ,
		peer  :  Addr<Peer> ,

	) -> Return<'static, ()>
	{
		let sid = msg.service();

		let receiver = match self.handlers.get( &sid )
		{
			Some(x) => x,

			None =>
			{
				let ctx = Peer::err_ctx( &peer, sid, msg.conn_id(), "Processing incoming Call".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::NoHandler{ ctx } ).boxed()
			}
		};

		match sid
		{
			$(
				_ if sid == *<$services as Service>::sid() =>
				{
					Self::call_service_gen::<$services>( msg, receiver, peer )
				}
			)+


			_ =>
			{
				let ctx = Peer::err_ctx( &peer, sid, msg.conn_id(), "Processing incoming Call".to_string() );

				return Self::handle_err( peer, ThesRemoteErr::UnknownService{ ctx } ).boxed()
			}
		}
	}
}



/// Concrete type for creating recipients for remote Services in this thespis::ServiceMap.
//
#[ derive( Clone, Debug ) ]
//
pub struct RemoteAddr
{
	// TODO: do not rely on Addr, we should be generic over Address, but not
	//       choose an implementation.
	//
	peer: Pin<Box< Addr<Peer> >>
}


impl RemoteAddr
{
	pub fn new( peer: Addr<Peer> ) -> Self
	{
		Self { peer: Box::pin( peer ) }
	}


	/// Take the raw message and turn it into a MultiService
	//
	fn build_ms<S>( msg: S, cid: ConnID ) -> Result< WireFormat, ThesRemoteErr >

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let sid  = <S as Service>::sid().clone();
		let sid2 = sid.clone();

		let serialized: Vec<u8> = serde_cbor::to_vec( &msg )

			.map_err( |_|
		{
			let mut ctx = ErrorContext::default();
			ctx.context = "Outgoing request".to_string().into();
			ctx.sid     = sid.into();

			ThesRemoteErr::Serialize{ ctx }

		})?;


		Ok( WireFormat::create( sid2, cid, serialized.into() ) )
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
	fn call( &mut self, msg: S ) -> Return<Result< <S as Message>::Return, ThesRemoteErr >> { async move
	{

		let cid = ConnID::random();

		// Serialization can fail
		//
		let call = Call::new( Self::build_ms( msg, cid.clone() )? );

		// Can fail if the peer is down already.
		//
		let rx = self.peer.call( call ).await?

			// The actual sending out over the network can fail.
			//
			.map_err( |_|
			{
				let ctx = ErrorContext
				{
					context  : Some( "Call remote service".to_string() ) ,
					peer_id  : self.peer.id().into()                     ,
					peer_name: self.peer.name()                          ,
					sid      : <S as Service>::sid().clone().into()      ,
					cid      : cid.clone().into()                        ,
				};

				ThesRemoteErr::ConnectionClosed{ ctx }

			})?;


		// Channel can be canceled
		//
		let re = rx.await

			.map_err( |_|
			{
				let ctx = ErrorContext
				{
					context  : Some( "Peer stopped before receiving response from remote call".to_string() ) ,
					peer_id  : self.peer.id().into()                                                         ,
					peer_name: self.peer.name()                                                              ,
					sid      : <S as Service>::sid().clone().into()                                          ,
					cid      : cid.clone().into()                                                            ,
				};

				ThesRemoteErr::ConnectionClosed{ ctx }

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
						let ctx = ErrorContext
						{
							context  : Some( "Response to call from remote actor".to_string() ) ,
							peer_id  : self.peer.id().into()                                    ,
							peer_name: self.peer.name()                                         ,
							sid      : <S as Service>::sid().clone().into()                     ,
							cid      : cid.clone().into()                                       ,
						};

						ThesRemoteErr::Deserialize{ ctx }

					})?
				)
			},

			// The remote returned an error
			//
			Err( err ) =>
			{
				let ctx = ErrorContext
				{
					context  : Some( "Remote could not process our message".to_string() ) ,
					peer_id  : self.peer.id().into()                                      ,
					peer_name: self.peer.name()                                           ,
					sid      : <S as Service>::sid().clone().into()                       ,
					cid      : cid.into()                                                 ,
				};

				Err( ThesRemoteErr::Remote{ err, ctx } )
			},
		}

	}.boxed() }


	/// Obtain a clone of this recipient as a trait object.
	//
	fn clone_box( &self ) -> BoxAddress<S, ThesRemoteErr>
	{
		Box::new( Self { peer: self.peer.clone() } )
	}


	/// Unique id of the peer this sends over
	//
	fn actor_id( &self ) -> usize
	{
		// TODO: self.peer.id()? and name?
		//
		< Addr<Peer> as Address<WireFormat> >::actor_id( &self.peer )
	}
}




impl<S> Sink<S> for RemoteAddr

	where  S                    : Service + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,


{
	type Error = ThesRemoteErr;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		< Addr<Peer> as Sink<WireFormat> >::poll_ready( self.peer.as_mut(), cx )

			.map_err( Into::into )
	}


	fn start_send( mut self: Pin<&mut Self>, msg: S ) -> Result<(), Self::Error>
	{
		< Addr<Peer> as Sink<WireFormat> >::start_send( self.peer.as_mut(), Self::build_ms( msg, ConnID::null() )? )

			.map_err( |_|
			{
				let ctx = ErrorContext
				{
					context  : Some( "Send to remote service".to_string() ) ,
					peer_id  : self.peer.id().into()                        ,
					peer_name: self.peer.name()                             ,
					sid      : <S as Service>::sid().clone().into()         ,
					cid      : None                                         ,
				};

				ThesRemoteErr::ConnectionClosed{ ctx }
			})
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		< Addr<Peer> as Sink<WireFormat> >::poll_flush( self.peer.as_mut(), cx )

			.map_err( Into::into )
	}


	/// Will only close when dropped, this method can never return ready
	//
	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		Poll::Pending
	}
}



}}} // End of macro
