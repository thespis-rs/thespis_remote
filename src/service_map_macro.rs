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
///       pub fn recipient<S>( peer: Addr<Peer> ) -> impl Recipient<S> {...}
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
///     // Some types to make the impl Recipient<S> in Services::recipient above.
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
	super :: { $( $services, )+ Peer                         } ,
	$crate:: { *                                             } ,
	std   :: { pin::Pin, collections::HashMap, fmt, any::Any } ,

	$crate::external_deps::
	{
		async_runtime   :: { rt                                           } ,
		once_cell       :: { sync::OnceCell                               } ,
		futures         :: { future::FutureExt, task::{ Context, Poll }   } ,
		thespis         :: { *                                            } ,
		thespis_remote  :: { *                                            } ,
		thespis_impl    :: { Addr, Receiver, ThesErr, ThesRes             } ,
		serde_cbor      :: { self, from_slice as des                      } ,
		serde           :: { Serialize, Deserialize, de::DeserializeOwned } ,
		log             :: { error                                        } ,
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
			static INSTANCE: OnceCell< ServiceID > = OnceCell::new();

			INSTANCE.get_or_init( ||
			{
				ServiceID::from_seed( stringify!( $ns::$services ).as_bytes() )
			})
		}
	}
)+


/// The actual service map.
/// Use it to get a recipient to a remote service.
//
pub struct Services
{
	handlers: HashMap< &'static ServiceID, Box< dyn Any + Send + Sync > >
}



/// TODO: Add unit test
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

		let mut accu = String::new();

		$(
			let sid = <$services as Service>::sid();

			accu += &format!
			(
				"\t{:width$} - sid: {:?} - handler (actor_id): {}\n",

				stringify!( $services ),
				sid,

				if let Some(h) = self.handlers.get( sid )
				{
					let handler: &Receiver<$services> = h.downcast_ref().expect( "downcast receiver in Debug for Services" );
					format!( "{:?}", handler.actor_id() )
				}

				else
				{
					"none".to_string()
				},

				width = width
			);
		)+

		write!( f, "{}::Services\n{{\n{}}}", stringify!( $ns ), accu )
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
		Self{ handlers: HashMap::new() }
	}


	/// Creates a recipient to a Service type for a remote actor, which can be used in exactly the
	/// same way as if the actor was local.
	//
	pub fn recipient<S>( peer: Addr<Peer> ) -> RemoteAddr

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		RemoteAddr::new( peer )
	}


	/// Register a handler for a given service type
	/// Calling this method twice for the same type will override the first handler.
	//
	pub fn register_handler<S>( &mut self, handler: Receiver<S> )

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,
	{
		self.handlers.insert( <S as Service>::sid(), Box::new( handler ) );
	}


	// Helper function for send_service below
	//
	fn send_service_gen<S>( msg: MultiServiceImpl, receiver: &Box< dyn Any + Send + Sync > ) -> ThesRemoteRes<()>

		where  S                    : Service + Send,
	         <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let backup: &Receiver<S> = receiver.downcast_ref()

			.ok_or( ThesRemoteErr::Downcast( "Receiver in service_macro".into() ))?
		;


		let message: S = des( &msg.mesg() )

			.map_err( |_| ThesRemoteErr::Deserialize( "Deserialize incoming remote message".into() ))?
		;


		let mut rec = backup.clone_box();

		rt::spawn( async move
		{
			// TODO: we are only logging the potential error. Can we do better? Note that spawn requires
			//       we return a tuple.
			//
			let res = rec.send( message ).await;

			if res.is_err() { error!( "Failed to deliver remote message to actor: {:?}", &rec ); }

		}).map_err( |_| -> ThesRemoteErr
		{
			ThesErr::Spawn{ actor: "Task for sending to the local Service".to_string() }.into()

		})?;

		Ok(())
	}



	// Helper function for call_service below
	//
	fn call_service_gen<S>
	(
		    msg        :  MultiServiceImpl                        ,
		    receiver   : &Box< dyn Any + Send + Sync >            ,
		mut return_addr:  BoxRecipient<MultiServiceImpl, ThesErr> ,

	) -> ThesRemoteRes<()>

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		// for reasoning on the expect, see send_service_gen
		//
		let backup: &Receiver<S> = receiver.downcast_ref()

			.ok_or( ThesRemoteErr::Downcast( "Receiver in service_macro".into() ))?
		;

		let message: S = des( &msg.mesg() )

			.map_err( |_| ThesRemoteErr::Deserialize( "Deserialize incoming remote message".into() ))?
		;

		let mut rec  = backup.clone_box() ;
		let cid_null = ConnID::null()     ;
		let cid      = msg.conn_id()?     ;


		rt::spawn( async move
		{
			// Call the service
			// TODO: we should probably match this error later. If it is a mailbox full, we could
			//       try again.
			//
			let resp = match Self::handle_err
			(
				&mut return_addr                                                                                       ,
				&cid                                                                                                   ,
				rec.call( message ).await.map_err( |e| { error!( "{:?}", e ); ConnectionError::InternalServerError } ) ,

			).await
			{
				Ok (resp) => resp   ,
				Err(_   ) => return ,
			};

			let bytes = serde_cbor::to_vec( &resp ).map_err( |e| { error!( "{:?}", e ); ConnectionError::Serialize } );

			// Serialize the response
			//
			let serialized = match Self::handle_err
			(
				&mut return_addr  ,
				&cid              ,
				bytes             ,

			).await
			{
				Ok (ser) => ser    ,
				Err(_  ) => return ,
			};


			// Create a MultiService
			//
			let     sid          = <S as Service>::sid().clone()                                   ;
			let     mul          = MultiServiceImpl::create( sid, cid.clone(), serialized.into() ) ;
			let mut return_addr2 = return_addr.clone_box()                                         ;

			// Send the MultiService out over the network.
			// We're returning anyway, so there's nothing to do with the result.
			//
			let _ = Self::handle_err
			(
				&mut return_addr                                                                                            ,
				&cid                                                                                                        ,
				return_addr2.send( mul ).await.map_err( |e| { error!( "{:?}", e ); ConnectionError::InternalServerError } ) ,

			).await;

		}).map_err( |_| -> ThesRemoteErr
		{
			ThesErr::Spawn{ actor: "Task for calling the local Service".to_string() }.into()

		})?;

		Ok(())
	}


	// Helper function for error handling:
	// - log the error
	// - send it to the remote
	// - close the connection
	//
	async fn handle_err<'a, T>
	(
		peer: &'a mut BoxRecipient<MultiServiceImpl, ThesErr> ,
		cid : &'a ConnID                                      ,
		res : Result<T, ConnectionError>                      ,

	) -> Result<T, ConnectionError>
	{
		match res
		{
			Ok (t) => Ok(t),
			Err(e) =>
			{
				let ms  = Peer::prep_error( cid.clone(), &e );
				let res = peer.send( ms ).await;

				if let Err( ref err ) = res { error!( "Failed to send error back to remote: {:?}", err ) };

				// TODO: In any case, close the connection.
				// we will need access to the peer other than the Recipient<MS>
				//
				// let res2 = peer.send( CloseConnection{ remote: false } ).await;

				// if let Err( ref err ) = res2 { error!( "Failed to close connection: {:?}", err ) };


				// return the result
				//
				Err(e)
			}
		}
	}
}



impl ServiceMap for Services
{
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
	/// - ThesRemoteErr::ThesErr -> Spawn error
	///
	/// # Panics
	/// For the moment this can panic if the downcast to Receiver fails. It should never happen unless there
	/// is a programmer error, but even then, it should be type checked, so for now I have decided to leave
	/// the expect in there. See if anyone manages to trigger it, we can take it from there.
	///
	//
	fn send_service( &self, msg: MultiServiceImpl ) -> ThesRemoteRes<()>
	{
		let sid = msg.service().expect( "get service" );

		match sid
		{
			$(
				_ if sid == *<$services as Service>::sid() =>
				{
					let receiver = self.handlers.get( &sid ).ok_or
					(
						ThesRemoteErr::UnknownService( format!( "sid: {:?}", sid ))
					)?;

					Self::send_service_gen::<$services>( msg, receiver )?
				},
			)+

			_ => Err( ThesRemoteErr::UnknownService( format!( "sid: {:?}", sid )) )?,
		};

		Ok(())
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
		&self                                                 ,
		msg        :  MultiServiceImpl                        ,
		return_addr:  BoxRecipient<MultiServiceImpl, ThesErr> ,

	) -> ThesRemoteRes<()>

	{
		let sid = msg.service().expect( "get service" );

		match sid
		{
			$(
				_ if sid == *<$services as Service>::sid() =>
				{
					let receiver = self.handlers.get( &sid ).ok_or
					(
						ThesRemoteErr::UnknownService( format!( "sid: {:?}", sid ))
					)?;

					Self::call_service_gen::<$services>( msg, receiver, return_addr )?
				},
			)+


			_ => Err( ThesRemoteErr::UnknownService( format!( "sid: {:?}", sid )) )?,
		};

		Ok(())
	}
}



/// Concrete type for creating recipients for remote Services in this thespis::ServiceMap.
//
#[ derive( Clone, Debug ) ]
//
pub struct RemoteAddr
{
	// TODO: get rid of boxing
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
	fn build_ms<S>( msg: S, cid: ConnID ) -> Result< MultiServiceImpl, ThesRemoteErr >

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let sid = <S as Service>::sid().clone();

		let serialized: Vec<u8> = serde_cbor::to_vec( &msg )

			.map_err( |_| ThesRemoteErr::Serialize( format!( "Service: {:?}", sid ) ) )?;


		Ok( MultiServiceImpl::create( sid, cid, serialized.into() ) )
	}



	// Actual impl for send
	//
	async fn send_gen<S>( &mut self, msg: S ) -> Result<(), ThesRemoteErr>

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		// build_ms errs if serialization fails.
		//
		let serialized = Self::build_ms( msg, ConnID::null() )?;

		// Addr<Peer> send can err if the channel is closed.
		//
		self.peer.send( serialized ).await.map_err( Into::into )
	}



	// potential errors:
	// 1. serialization of the outgoing message
	// 2.
	async fn call_gen<S>( &mut self, msg: S ) -> Result< <S as Message>::Return, ThesRemoteErr >

		where  S                    : Service + Send,
		      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		// Serialization can fail
		//
		let call = Call::new( Self::build_ms( msg, ConnID::random() )? );

		// Call can fail (normally only if the thread in which the mailbox lives craches), or TODO: when we will
		// use bounded channels.
		//
		let rx = self.peer.call( call ).await??;

		// Channel can be canceled
		//
		let re = rx.await.map_err( |e| ThesErr::MailboxClosedBeforeResponse{ actor: "Peer".into() } )?;


		// A response came back from the other side.
		//
		match re
		{
			Ok ( resp ) =>
			{
				// deserialize the payload and return it to the caller.
				// Deserialization can fail.
				//
				Ok( des( &resp.mesg() )

					.map_err( |_| ThesRemoteErr::Deserialize( "response from remote actor".into() ) )? )
			},

			// The remote returned an error
			//
			Err( err ) =>
			{
				Err( ThesRemoteErr::Connection ( format!( "Remote could not process our call correctly: {:?}", err ) ) )
			},
		}
	}
}




impl<S> Recipient<S> for RemoteAddr

	where  S                    : Service + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,

{
	/// Call a remote actor.
	//
	fn call( &mut self, msg: S ) -> Return<Result< <S as Message>::Return, ThesRemoteErr >>
	{
		Box::pin( self.call_gen( msg ) )
	}


	/// Obtain a clone of this recipient as a trait object.
	//
	fn clone_box( &self ) -> BoxRecipient<S, ThesRemoteErr>
	{
		Box::new( Self { peer: self.peer.clone() } )
	}


	/// Unique id of the peer this sends over
	//
	fn actor_id( &self ) -> usize
	{
		< Addr<Peer> as Recipient<MultiServiceImpl> >::actor_id( &self.peer )
	}
}




impl<S> Sink<S> for RemoteAddr

	where  S                    : Service + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,


{
	type Error = ThesRemoteErr;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		< Addr<Peer> as Sink<MultiServiceImpl> >::poll_ready( self.peer.as_mut(), cx )

			.map_err( Into::into )
	}


	fn start_send( mut self: Pin<&mut Self>, msg: S ) -> Result<(), Self::Error>
	{
		< Addr<Peer> as Sink<MultiServiceImpl> >::start_send( self.peer.as_mut(), Self::build_ms( msg, ConnID::null() )? )

			.map_err( Into::into )
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::Error>>
	{
		< Addr<Peer> as Sink<MultiServiceImpl> >::poll_flush( self.peer.as_mut(), cx )

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
