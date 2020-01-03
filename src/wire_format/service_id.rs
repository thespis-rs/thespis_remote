use
{
	crate :: { import::*, ThesRemoteErr },
	super :: { unique_id::UniqueID      },
};

/// A unique identifier for a service that is exposed to other processes. This will allow
/// identifying the type to which the payload needs to be deserialized and the actor to which
/// this message is to be delivered.
///
/// Ideally we want to use 128 bits here to have globally unique identifiers with little chance
/// of collision, but we use xxhash which for the moment only supports 64 bit.
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
	pub fn from_seed( data: &[u8] ) -> Self
	{
		Self{ inner: UniqueID::from_seed( data ) }
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
impl TryFrom< Bytes > for ServiceID
{
	type Error = ThesRemoteErr;

	fn try_from( bytes: Bytes ) -> Result<Self, ThesRemoteErr>
	{
		UniqueID::try_from( bytes )

			.map( |inner| Self{ inner } )
	}
}


impl fmt::Display for ServiceID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "{:?}", self )
	}
}



impl fmt::Debug for ServiceID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		self.inner.fmt( f )
	}
}
