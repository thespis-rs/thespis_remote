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
/// Types created by this macro, for the following invocation:
///
/// ```ignore
/// type MySink = SplitSink<...>;
/// type MyPeer = Peer<MultiServiceImpl, MySink>;
/// service_map!
/// (
///    namespace: myns                 ;
///    peer_type: MyPeer               ;
///    multi_service: MultiServiceImpl ;
///
///    services:
///
///    	ServiceA,
///    	ServiceB,
/// );
///
/// mod myns
/// {
///    pub trait MarkServices {}; // trait bound for all services in this service_map
///
///    impl MarkServices for ServiceA {}
///    impl MarkServices for ServiceB {}
///
///    // sid will be different for ServiceA in another service map with another namespace than myns
///    //
///    impl Service<self::Services> for ServiceA {...} // self being myns
///    impl Service<self::Services> for ServiceB {...}
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
///       pub fn recipient<S>( peer: Addr<MyPeer> ) -> impl Recipient<S> {...}
///
///       ...
///     }
///
///     // Service map is defined in the thespis crate. This is for the network peer, normally you don't
///     // need to use this directly as a user.
///     //
///     impl ServiceMap<MultiServiceImpl> for Services {...}
///
///     // Some types to make the impl Recipient<S> in Services::recipient above.
/// }
/// ```
///
/// TODO: - this is not generic right now, cbor is hardcoded
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

	/// The type used for [Peer]. This is because [Peer] is generic, thus you need to specify the exact type.
	//
	peer_type: $peer_type: path;

	/// The type you want to use that implements [thespis::MultiService] (the wire format).
	//
	multi_service: $ms_type: path;

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
	super :: { $( $services, )+ $peer_type, $ms_type } ,
	$crate:: { *                                     } ,
	std   :: { pin::Pin                              } ,

	$crate::external_deps::
	{
		once_cell       :: { sync::OnceCell                               } ,
		futures         :: { future::FutureExt, task::{ Context, Poll }   } ,
		thespis         :: { *                                            } ,
		thespis_remote  :: { *                                            } ,
		thespis_impl    :: { runtime::rt, Addr, Receiver                  } ,
		serde_cbor      :: { self, from_slice as des                      } ,
		serde           :: { Serialize, Deserialize, de::DeserializeOwned } ,
		failure         :: { Fail, ResultExt                              } ,
		log             :: { error                                        } ,
	},
};


type ConnID    = <$ms_type as MultiService>::ConnID    ;
type ServiceID = <$ms_type as MultiService>::ServiceID ;
// type Codecs    = <$ms_type as MultiService>::CodecAlg  ;

/// Mark the services part of this particular service map, so we can write generic impls only for them.
//
pub trait MarkServices {}

$(

	impl MarkServices for $services {}

	impl Service<self::Services> for $services
	{
		type UniqueID = ServiceID;

		fn sid() -> &'static Self::UniqueID
		{
			static INSTANCE: OnceCell< ServiceID > = OnceCell::INIT;

			INSTANCE.get_or_init( ||
			{
				ServiceID::from_seed( &[ stringify!( $services ), Services::NAMESPACE ].concat().as_bytes() )
			})
		}
	}
)+


/// The actual service map.
/// Use it to get a recipient to a remote service.
///
#[ derive( Clone ) ]
//
pub struct Services;

impl Namespace for Services { const NAMESPACE: &'static str = stringify!( $ns ); }


impl Services
{
	/// Creates a recipient to a Service type for a remote actor, which can be used in exactly the
	/// same way as if the actor was local.
	//
	pub fn recipient<S>( peer: Addr<$peer_type> ) -> impl Recipient<S>

	where  S:   MarkServices + Serialize + DeserializeOwned + Send
	          + Service<self::Services, UniqueID=ServiceID>,

	      <S as Message>::Return: Serialize + DeserializeOwned + Send,
	{
		ServicesRecipient::new( peer )
	}



	// Helper function for send_service below
	//
	fn send_service_gen<S>( msg: $ms_type, receiver: &BoxAny ) -> ThesRemoteRes<()>

		where  S:   MarkServices + Serialize + DeserializeOwned + Send
		          + Service<self::Services, UniqueID=ServiceID>,

			      <S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let backup: &Receiver<S> = receiver.downcast_ref()

			.ok_or( ThesRemoteErrKind::Downcast( "Receiver in service_macro".into() ))?
		;


		let message = des( &msg.mesg() )

			.context( ThesRemoteErrKind::Deserialize( "Deserialize incoming remote message".into() ))?
		;


		let mut rec = backup.clone_box();

		rt::spawn( async move
		{
			// TODO: we are only logging the potential error. Can we do better? Note that spawn requires
			//       we return a tuple.
			//
			let res = await!( rec.send( message ) );

			error!( "Failed to deliver remote message to actor: {:?}", &rec );

		}).context( ThesRemoteErrKind::ThesErr( "Spawn task for sending to the local Service".into() ))?;

		Ok(())
	}



	// Helper function for call_service below
	//
	fn call_service_gen<S>
	(
		     msg        :  $ms_type               ,
		     receiver   : &BoxAny                 ,
		 mut return_addr:  BoxRecipient<$ms_type> ,

	) -> ThesRemoteRes<()>

	where S:   MarkServices + Serialize + DeserializeOwned + Send
	         + Service<self::Services, UniqueID=ServiceID>,

	      <S as Message>::Return: Serialize + DeserializeOwned + Send,
	{
		// for reasoning on the expect, see send_service_gen
		//
		let backup: &Receiver<S> = receiver.downcast_ref()

			.ok_or( ThesRemoteErrKind::Downcast( "Receiver in service_macro".into() ))?
		;

		let message = des( &msg.mesg() )

			.context( ThesRemoteErrKind::Deserialize( "Deserialize incoming remote message".into() ))?
		;

		let mut rec  = backup.clone_box();
		let cid_null = ConnID::null() ;
		let cid      = msg.conn_id()?                             ;


		rt::spawn( async move
		{
			// Call the service
			// TODO: we should probably match this error later. If it is a mailbox full, we could
			//       try again.
			//
			let resp = match await!( Self::handle_err
			(
				&mut return_addr                                                                                           ,
				&cid                                                                                                       ,
				await!( rec.call( message ) ).map_err( |e| { error!( "{:?}", e ); ConnectionError::InternalServerError } ) ,
			))
			{
				Ok (resp) => resp   ,
				Err(_   ) => return ,
			};


			// Serialize the response
			//
			let serialized = match await!( Self::handle_err
			(
				&mut return_addr                                                                               ,
				&cid                                                                                           ,
				serde_cbor::to_vec( &resp ).map_err( |e| { error!( "{:?}", e ); ConnectionError::Serialize } ) ,
			))
			{
				Ok (ser) => ser    ,
				Err(_  ) => return ,
			};


			// Create a MultiService
			//
			let     sid          = <S as Service<self::Services>>::sid().clone()                           ;
			let     mul          = <$ms_type>::create( sid, cid.clone(), Codecs::CBOR, serialized.into() ) ;
			let mut return_addr2 = return_addr.clone_box()                                                 ;

			// Send the MultiService out over the network.
			// We're returning anyway, so there's nothing to do with the result.
			//
			let _ = await!( Self::handle_err
			(
				&mut return_addr                                                                                                ,
				&cid                                                                                                            ,
				await!( return_addr2.send( mul ) ).map_err( |e| { error!( "{:?}", e ); ConnectionError::InternalServerError } ) ,
			));

		}).context( ThesRemoteErrKind::ThesErr( "Spawn task for calling the local Service".into() ))?;

		Ok(())
	}


	// Helper function for error handling:
	// - log the error
	// - send it to the remote
	// - close the connection
	//
	async fn handle_err<'a, T>
	(
		peer: &'a mut BoxRecipient<$ms_type>         ,
		cid : &'a ConnID ,
		res : Result<T, ConnectionError>             ,

	) -> Result<T, ConnectionError>
	{
		match res
		{
			Ok (t) => Ok(t),
			Err(e) =>
			{
				let ms  = <$peer_type>::prep_error( cid.clone(), &e );
				let res = await!( peer.send( ms ) );

				if let Err( ref err ) = res { error!( "Failed to send error back to remote: {:?}", err ) };

				// TODO: In any case, close the connection.
				// we will need access to the peer other than the Recipient<MS>
				//
				// let res2 = await!( peer.send( CloseConnection{ remote: false } ) );

				// if let Err( ref err ) = res2 { error!( "Failed to close connection: {:?}", err ) };


				// return the result
				//
				Err(e)
			}
		}
	}
}



impl ServiceMap<$ms_type> for Services
{
	fn boxed() -> BoxServiceMap<MS>
	{
		box Self
	}


	/// Will match the type of the service id to deserialize the message and send it to the handling actor.
	///
	/// This can return the following errors:
	/// - ThesRemoteErrKind::Downcast
	/// - ThesRemoteErrKind::UnknownService
	/// - ThesRemoteErrKind::Deserialize
	/// - ThesRemoteErrKind::ThesErr -> Spawn error
	///
	/// # Panics
	/// For the moment this can panic if the downcast to Receiver fails. It should never happen unless there
	/// is a programmer error, but even then, it should be type checked, so for now I have decided to leave
	/// the expect in there. See if anyone manages to trigger it, we can take it from there.
	///
	//
	fn send_service( &self, msg: $ms_type, receiver: &BoxAny ) -> ThesRemoteRes<()>
	{
		let x = msg.service().expect( "get service" );

		match x
		{
			$(
				_ if x == *<$services as Service<self::Services>>::sid() =>

					Self::send_service_gen::<$services>( msg, receiver )? ,
			)+

			_ => Err( ThesRemoteErrKind::UnknownService( format!( "sid: {:?}", x )) )?,
		};

		Ok(())
	}



	/// Will match the type of the service id to deserialize the message and call the handling actor.
	///
	/// This can return the following errors:
	/// - ThesRemoteErrKind::Downcast
	/// - ThesRemoteErrKind::UnknownService
	/// - ThesRemoteErrKind::Deserialize
	/// - ThesRemoteErrKind::ThesErr -> Spawn error
	///
	/// # Panics
	/// For the moment this can panic if the downcast to Receiver fails. It should never happen unless there
	/// is a programmer error, but even then, it should be type checked, so for now I have decided to leave
	/// the expect in there. See if anyone manages to trigger it, we can take it from there.
	///
	//
	fn call_service
	(
		&self                                 ,
		 msg        :  $ms_type               ,
		 receiver   : &BoxAny                 ,
		 return_addr:  BoxRecipient<$ms_type> ,

	) -> ThesRemoteRes<()>

	{
		let x = msg.service().expect( "get service" );

		match x
		{
			$(
				_ if x == *<$services as Service<self::Services>>::sid() =>

					Self::call_service_gen::<$services>( msg, receiver, return_addr )? ,
			)+


			_ => Err( ThesRemoteErrKind::UnknownService( format!( "sid: {:?}", x )) )?,
		};

		Ok(())
	}
}



/// Concrete type for creating recipients for remote Services in this thespis::ServiceMap.
//
#[ derive( Clone, Debug ) ]
//
pub struct ServicesRecipient
{
	peer: Pin<Box< Addr<$peer_type> >>
}


impl ServicesRecipient
{
	pub fn new( peer: Addr<$peer_type> ) -> Self
	{
		Self { peer: Box::pin( peer ) }
	}


	/// Take the raw message and turn it into a MultiService
	//
	fn build_ms<S>( msg: S, cid: ConnID ) -> ThesRes< $ms_type >

		where  S: Service<self::Services, UniqueID=ServiceID> + Send,
				<S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let sid = <S as Service<self::Services>>::sid().clone();

		let serialized: Vec<u8> = serde_cbor::to_vec( &msg )

			.context( ThesErrKind::Serialize{ what: format!( "Service: {:?}", sid ) } )?.into();


		Ok( <$ms_type>::create( sid, cid, Codecs::CBOR, serialized.into() ) )
	}


	// Actual impl for send
	//
	async fn send_gen<S>( &mut self, msg: S ) -> ThesRes<()>

		where  S: Service<self::Services, UniqueID=ServiceID> + Send,
				<S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		await!( self.peer.send( Self::build_ms( msg, ConnID::null() )? ) )
	}


	async fn call_gen<S>( &mut self, msg: S ) -> ThesRes<<S as Message>::Return>

		where  S: Service<self::Services, UniqueID=ServiceID> + Send,
				<S as Message>::Return: Serialize + DeserializeOwned + Send,

	{
		let call = Call::new( Self::build_ms( msg, ConnID::default() )? );

		let re = match await!( self.peer.call( call ) )?
		{
			Ok (rx) => await!( rx ).context( ThesErrKind::MailboxClosedBeforeResponse{ actor: "Peer".into() } )?,

			Err(e ) => match e.kind()
			{
				ThesRemoteErrKind::ConnectionClosed{..} =>
				{
					Err( e.kind().clone().context
					(
						ThesErrKind::MailboxClosed{ actor: format!( "{:?}", self.peer ) }
					))?
				},

				ThesRemoteErrKind::Deserialize{..} =>
				{
					Err( e.kind().clone().context
					(
						ThesErrKind::Deserialize{ what : format!( "{:?}", self.peer ) }
					))?
				},

				_ => unreachable!(),
			}
		};

		Ok( des( &re.mesg() ).context( ThesErrKind::Deserialize{ what: "response from remote actor".to_string() } )? )
	}
}




impl<S> Recipient<S> for ServicesRecipient

	where  S: MarkServices + Service<self::Services, UniqueID=ServiceID> + Serialize + DeserializeOwned + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,

{
	/// Call a remote actor.
	//
	fn call( &mut self, msg: S ) -> Return< ThesRes<<S as Message>::Return> >
	{
		Box::pin( self.call_gen( msg ) )
	}


	/// Obtain a clone of this recipient as a trait object.
	//
	fn clone_box( &self ) -> BoxRecipient<S>
	{
		box Self { peer: self.peer.clone() }
	}


	/// Unique id of the peer this sends over
	//
	fn actor_id( &self ) -> usize
	{
		< Addr<$peer_type> as Recipient<$ms_type> >::actor_id( &self.peer )
	}
}




impl<S> Sink<S> for ServicesRecipient

	where  S: MarkServices + Service<self::Services, UniqueID=ServiceID> + Serialize + DeserializeOwned + Send,
	      <S as Message>::Return: Serialize + DeserializeOwned + Send,


{
	type SinkError = ThesErr;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::SinkError>>
	{
		<Addr<$peer_type> as Sink<$ms_type>>::poll_ready( self.peer.as_mut(), cx )
	}


	fn start_send( mut self: Pin<&mut Self>, msg: S ) -> Result<(), Self::SinkError>
	{
		<Addr<$peer_type> as Sink<$ms_type>>::start_send( self.peer.as_mut(), Self::build_ms( msg, ConnID::null() )? )
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::SinkError>>
	{
		<Addr<$peer_type> as Sink<$ms_type>>::poll_flush( self.peer.as_mut(), cx )
	}


	/// Will only close when dropped, this method can never return ready
	//
	fn poll_close( mut self: Pin<&mut Self>, cx: &mut Context ) -> Poll<Result<(), Self::SinkError>>
	{
		Poll::Pending
	}
}



}}} // End of macro
