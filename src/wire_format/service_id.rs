use
{
	crate :: { import::*           } ,
	super :: { unique_id::UniqueID } ,
};


static SERVICES: SyncLazy<Mutex< HashMap<&'static ServiceID, &'static str> >> = SyncLazy::new( ||

	Mutex::new( HashMap::new() )
);

/// A unique identifier for a service that is exposed to other processes. This will allow
/// identifying the type to which the payload needs to be deserialized and the actor to which
/// this message is to be delivered.
///
/// Ideally we want to use 128 bits here to have globally unique identifiers with little chance
/// of collision, but we use xxhash which for the moment only supports 64 bit, so we hash the
/// namespace and typename separately both to 64 bits.
///
/// 2 values are reserved, all zero's and all one's are used as special values by Peer to
/// detect error conditions. If ever your namespace + typename would hash to one of these,
/// please change them.
//
#[ derive( Clone, PartialEq, Eq, Hash, Serialize, Deserialize ) ]
//
pub struct ServiceID
{
	inner: UniqueID,
}


impl ServiceID
{
	/// Seed the ServiceID. It might be data that will be hashed to generate the id.
	/// An identical input here should always give an identical ServiceID.
	//
	pub fn from_seed( namespace: &[u8], typename: &[u8] ) -> Self
	{
		let inner = UniqueID::from_seed( namespace, typename );

		debug_assert!( !inner.is_null(), "Hashing your namespace + typename generated a hash that is all zero's, which is a reserved value. Please slightly change either one." );

		Self{ inner }
	}


	/// And empty ServiceID. Can be used to signify the abscence of an id, would usually be all
	/// zero bytes.
	//
	pub fn null() -> Self
	{
		Self{ inner: UniqueID::null() }
	}


	/// Predicate for null values (all bytes are 0).
	//
	pub fn is_null( &self ) -> bool
	{
		self.inner.is_null()
	}


	/// A full ServiceID. Value reserved by thespis to detect an error condition.
	//
	pub fn full() -> Self
	{
		Self{ inner: UniqueID::full() }
	}


	/// Predicate for full values (all bytes are 1).
	//
	pub fn is_full( &self ) -> bool
	{
		self.inner.is_full()
	}


	/// Register the typename a ServiceID refers to so it can be used later for log output.
	/// the `service_map!` macro does this automatically for you.
	//
	pub fn register_service( sid: &'static ServiceID, name: &'static str )
	{
		let mut s = SERVICES.lock();

		// unwrap: OnceCell already guarantees us unique access.
		//
		s.entry( sid ).or_insert( name );
	}


	/// Look up the typename for a ServiceID.
	//
	pub fn service_name( sid: &ServiceID ) -> Option<&'static str>
	{
		let s = SERVICES.lock();

		// unwrap: OnceCell already guarantees us unique access.
		//
		s.get( sid ).copied()
	}
}



/// Internally is also represented as Bytes, so you just get a copy.
//
impl Into< Bytes > for ServiceID
{
	fn into( self ) -> Bytes
	{
		self.inner.into()
	}
}


/// The object will just keep the bytes as internal representation, no copies will be made
//
impl From< Bytes > for ServiceID
{
	fn from( bytes: Bytes ) -> Self
	{
		Self { inner: UniqueID::from( bytes ) }
	}
}


impl fmt::Display for ServiceID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match Self::service_name( self )
		{
			Some(name) => write!( f, "{}", name ),
			None       => self.inner.fmt( f ),
		}
	}
}



impl fmt::Debug for ServiceID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match Self::service_name( self )
		{
			Some(name) => write!( f, "ServiceID: {} ({:?})", name, self.inner ),
			None       => self.inner.fmt( f ),
		}
	}
}


impl fmt::LowerHex for ServiceID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		fmt::LowerHex::fmt( &self.inner, f )
	}
}
