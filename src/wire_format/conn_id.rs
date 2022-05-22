use
{
	crate :: { import::*                },
	super :: { unique_id::UniqueID      },
};

/// A unique identifier for a service that is exposed to other processes. This will allow
/// identifying the type to which the payload needs to be deserialized and the actor to which
/// this message is to be delivered.
///
/// Ideally we want to use 128 bits here to have globally unique identifiers with little chance
/// of collision, but we use xxhash which for the moment only supports 64 bit.
//
#[ derive( Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize ) ]
//
pub struct ConnID
{
	inner: UniqueID,
}


impl ConnID
{
	/// Generate a random ID
	//
	pub fn random() -> Self
	{
		Self { inner: UniqueID::random() }
	}


	/// And empty ConnID. Can be used to signify the abscence of an id, would usually be all
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
impl From< ConnID > for u64
{
	fn from( cid: ConnID ) -> u64
	{
		cid.inner.into()
	}
}


/// The object will just keep the bytes as internal representation, no copies will be made
//
impl From< u64 > for ConnID
{
	fn from( id: u64 ) -> Self
	{
		Self { inner: UniqueID::from( id ) }
	}
}


impl fmt::Display for ConnID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "{:?}", self )
	}
}



impl fmt::Debug for ConnID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		self.inner.fmt( f )
	}
}


impl fmt::LowerHex for ConnID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		fmt::LowerHex::fmt( &self.inner, f )
	}
}
