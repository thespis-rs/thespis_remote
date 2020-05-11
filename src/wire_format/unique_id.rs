use crate::{ import::* };


/// A unique identifier for a service that is exposed to other processes. This will allow
/// identifying the type to which the payload needs to be deserialized and the actor to which
/// this message is to be delivered.
///
/// Ideally we want to use 128 bits here to have globally unique identifiers with little chance
/// of collision, but we use xxhash which for the moment only supports 64 bit.
//
#[ derive( Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize ) ]
//
pub(crate) struct UniqueID
{
	id: u64,
}


impl UniqueID
{
	/// Generate a random ID
	//
	pub(crate) fn random() -> Self
	{
		let mut rng = rand::thread_rng();
		Self { id: rng.gen::<u64>() }
	}


	/// Seed the UniqueID. The data will be hashed with xxhash. The UniqueID is 128 bits and currently
	/// xxhash only supports hashing to 64bit output. We take 2 seeds, one for each 64bit hash.
	/// Parameter high will be the most significant bytes in the output in little endian (msb is highest offset).
	///
	/// An identical input here should always give an identical UniqueID.
	//
	pub(crate) fn from_seed( data: &[u8] ) -> Self
	{
		let mut h = XxHash64::default();

		h.write( data );

		Self { id: h.finish() }
	}


	/// And empty UniqueID. Can be used to signify the abscence of an id, would usually be all
	/// zero bytes.
	//
	pub(crate) fn null() -> Self
	{
		Self { id: 0 }
	}


	/// And full UniqueID. Reserved for use by thespis_remote, would usually be all
	/// one bytes.
	/// u128 still isn't well supported everywhere (WASM), so use 2 u64;
	//
	pub(crate) fn full() -> Self
	{
		Self { id: u64::MAX }
	}


	/// Predicate for null values (all bytes are 0).
	//
	pub(crate) fn is_null( &self ) -> bool
	{
		self.id == 0
	}


	/// Predicate for null values (all bytes are 0).
	//
	pub(crate) fn is_full( &self ) -> bool
	{
		self.id == u64::MAX
	}
}



/// Internally is also represented as Bytes, so you just get a copy.
//
impl Into< u64 > for UniqueID
{
	fn into( self ) -> u64
	{
		self.id
	}
}


/// The object will just keep the bytes as internal representation, no copies will be made
//
impl From< u64 > for UniqueID
{
	fn from( id: u64 ) -> Self
	{
		Self { id }
	}
}



impl fmt::Display for UniqueID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		<Self as fmt::Debug>::fmt( self, f )
	}
}



impl fmt::Debug for UniqueID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		// Padding so that it's always showing the leading zero if the first byte is single digit.
		// The 0x prefix is counted, hence 18 instead of 16.
		//
		write!( f, "{:#018x}", self )?;
		Ok(())
	}
}


impl fmt::LowerHex for UniqueID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		self.id.fmt( f )
	}
}


#[cfg(test)]
//
mod tests
{
	// What's tested:
	// 1. Identical input data should give identical hash.
	// 2. Creating full/null values create values that get detected by is_null, is_full predicates.
	// 3. debug output.
	//
	use super::{ *, assert_eq };


	#[test]
	//
	fn identical()
	{
		let sid  = UniqueID::from_seed( b"namespace::Typename" );
		let sid2 = UniqueID::from_seed( b"namespace::Typename" );

		assert_eq!( sid, sid2 );
	}


	#[test]
	//
	fn full()
	{
		let sid  = UniqueID::full();

		assert!( sid.is_full() );
	}


	#[test]
	//
	fn null()
	{
		let sid  = UniqueID::null();

		assert!( sid.is_null() );
	}


	#[test]
	//
	fn debug()
	{
		let sid = UniqueID{ id: 0x0f0e0d0c0b0a0908 };

		assert_eq!( "0x0f0e0d0c0b0a0908", &format!( "{:?}", sid ) );
	}
}


