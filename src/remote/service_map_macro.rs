/// This is a beefy macro which is your main interface to using the remote actors. It's unavoidable to
/// require code in the client application because thespis does not know the types of messages you will
/// create, yet we aim at making the difference between local and remote actors seamless for using code.
///
/// This macro will allow deserializing messages to the correct types, as well as creating recipients for
/// remote actors.
///
/// I have named the parameters for clarity, however it's a macro, and not real named parameters so the
/// order needs to be exact.
///
/// Please open declaration section to see the parameter documentation. The module [remote] has more
/// documentation on using remote actors, and there is a remote example and unit tests in the repository
/// which you can look at for a more fully fledged example.
///
/// TODO: document the types made available to the user by this macro.
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
	// It's important the comma be inside the parenthesis, because these lists might be empty, in which
	// we should not have a leading comma before the next item, but if the comma is after the closing
	// parenthesis, it will not output a trailing comma, which will be needed to separate from the next item.
	//
	super :: { $( $services, )+ $peer_type, $ms_type } ,
	$crate:: { *, remote::*, single_thread::*, runtime::rt                  } ,
	std   :: { any::Any                                                     } ,

	$crate::external_deps::
	{
		once_cell    :: { sync::OnceCell                               } ,
		futures      :: { future::FutureExt                            } ,
		thespis      :: { *                                            } ,
		serde_cbor   :: { self, from_slice as des                      } ,
		serde        :: { Serialize, Deserialize, de::DeserializeOwned } ,
	},
};

/// Mark the services part of this particular service map, so we can write generic impls only for them.
//
pub trait MarkServices {}

$
(
	impl MarkServices for $services {}

	impl Service<self::Services> for $services
	{
		type UniqueID = ServiceID;

		fn sid() -> &'static Self::UniqueID
		{
			static INSTANCE: OnceCell<ServiceID> = OnceCell::INIT;

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

	where  S                    : MarkServices + Service<self::Services, UniqueID=ServiceID> + Serialize + DeserializeOwned ,
	      <S as Message>::Return: Serialize + DeserializeOwned                                              ,
	{
		ServicesRecipient::new( peer )
	}



	// Helper function for send_service below
	//
	fn send_service_gen<S>( msg: $ms_type, receiver: &Box< dyn Any > )

		where  S                    : Service<self::Services> + Message ,
		      <S as Message>::Return: Serialize + DeserializeOwned      ,

	{
		let     backup: &Rcpnt<S> = receiver.downcast_ref().expect( "downcast_ref failed" );
		let mut rec               = backup.clone_box();

		let message = serde_cbor::from_slice( &msg.mesg() ).expect( "deserialize serviceA" );

		rt::spawn( async move
		{
			await!( rec.send( message ) ).expect( "call actor" );

		}).expect( "spawn call for service" );
	}



	// Helper function for call_service below
	//
	fn call_service_gen<S>
	(
		     msg        :  $ms_type                        ,
		     receiver   : &Box< dyn Any >                  ,
		 mut return_addr:  Box< dyn Recipient< $ms_type >> ,

	)

	where  S                    : Service<self::Services, UniqueID=ServiceID> ,
	      <S as Message>::Return: Serialize + DeserializeOwned,
	{
		let     backup: &Rcpnt<S> = receiver.downcast_ref().expect( "downcast_ref failed" );
		let mut rec               = backup.clone_box();

		let message = serde_cbor::from_slice( &msg.mesg() ).expect( "deserialize serviceA" );

		rt::spawn( async move
		{
			let resp       = await!( rec.call( message ) ).expect( "call actor" );

			// it's important that the sid is null here, because a response with both cid and sid
			// not null is interpreted as an error.
			//
			let sid        = <S as Service<self::Services>>::sid().clone();
			let cid        = msg.conn_id().expect( "get conn_id" );
			let serialized = serde_cbor::to_vec( &resp ).expect( "serialize response" );

			let mul        = MultiServiceImpl::new( sid, cid, Codecs::CBOR, serialized.into() );

			await!( return_addr.send( mul ) ).expect( "send response back to peer" );

		}).expect( "spawn call for service" );
	}
}



impl ServiceMap<$ms_type> for Services
{
	// Will match the type of the service id to deserialize the message and send it to the handling actor
	//
	fn send_service( &self, msg: $ms_type, receiver: &Box< dyn Any > )
	{
		let x = msg.service().expect( "get service" );

		match x
		{
			$(
				_ if x == *<$services as Service<self::Services>>::sid() =>

					Self::send_service_gen::<$services>( msg, receiver ) ,
			)+

			_ => panic!( "got wrong service: {:?}", x ),
		}
	}



	// Will match the type of the service id to deserialize the message and call the handling actor
	//
	fn call_service
	(
		&self                                          ,
		 msg        :  $ms_type                        ,
		 receiver   : &Box< dyn Any >                  ,
		 return_addr:  Box< dyn Recipient< $ms_type >> ,

	)
	{
		let x = msg.service().expect( "get service" );

		match x
		{
			$(
				_ if x == *<$services as Service<self::Services>>::sid() =>

					Self::call_service_gen::<$services>( msg, receiver, return_addr ) ,
			)+


			_ => panic!( "got wrong service: {:?}", x ),
		}
	}
}



/// Concrete type for creating recipients for remote Services in this thespis::ServiceMap.
//
#[ derive( Clone, Debug ) ]
//
pub struct ServicesRecipient
{
	peer: Addr<$peer_type>
}


impl ServicesRecipient
{
	pub fn new( peer: Addr<$peer_type> ) -> Self
	{
		Self { peer }
	}


	async fn send_gen<S>( &mut self, msg: S ) -> ThesRes<()>

		where  S                    : Service<self::Services, UniqueID=ServiceID> ,
				<S as Message>::Return: Serialize + DeserializeOwned,

	{
		let sid        = <S as Service<self::Services>>::sid().clone();
		let cid        = ConnID::null();
		let serialized = serde_cbor::to_vec( &msg )?;

		let mul        = MultiServiceImpl::new( sid, cid, Codecs::CBOR, serialized.into() );

		await!( self.peer.send( mul ) )
	}


	async fn call_gen<S>( &mut self, msg: S ) -> ThesRes<<S as Message>::Return>

		where  S                    : Service<self::Services, UniqueID=ServiceID> ,
				<S as Message>::Return: Serialize + DeserializeOwned,

	{
		let sid        = <S as Service<self::Services>>::sid().clone();
		let cid        = ConnID::default();
		let serialized = serde_cbor::to_vec( &msg )?.into();

		let mul        = MultiServiceImpl::new( sid, cid, Codecs::CBOR, serialized );

		let call = Call::new( mul );
		let re   = await!( await!( self.peer.call( call ) )?? )?;

		Ok( des( &re.mesg() )? )
	}
}




impl<S> Recipient<S> for ServicesRecipient

	where  S                    : MarkServices + Service<self::Services, UniqueID=ServiceID> + Serialize + DeserializeOwned ,
	      <S as Message>::Return: Serialize + DeserializeOwned                                              ,

{
	/// Send any thespis::Service message that has a impl Message to a remote actor.
	//
	fn send( &mut self, msg: S ) -> Return< ThesRes<()> >
	{
		self.send_gen( msg ).boxed()
	}


	/// Call a remote actor.
	/// TODO: does this return type have must use or should we specify that ourselves. Maybe on the
	///       type alias...
	//
	fn call( &mut self, msg: S ) -> Return< ThesRes<<S as Message>::Return> >
	{
		self.call_gen( msg ).boxed()
	}


	/// Obtain a clone of this recipient as a trait object.
	//
	fn clone_box( &self ) -> Box< dyn Recipient<S> >
	{
		box Self { peer: self.peer.clone() }
	}
}

}

}} // End of macro
