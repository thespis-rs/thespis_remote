use crate::{ import::* };


/// A unique identifier for a service that is exposed to other processes. This will allow
/// identifying the type to which the payload needs to be deserialized and the actor to which
/// this message is to be delivered.
///
/// Ideally we want to use 128 bits here to have globally unique identifiers with little chance
/// of collision, but we use xxhash which for the moment only supports 64 bit.
//
#[ derive( Clone, PartialEq, Eq, Hash ) ]
//
pub(crate) struct UniqueID
{
	bytes: Bytes,
}


impl UniqueID
{
	/// Generate a random ID
	//
	pub(crate) fn random() -> Self
	{
		let mut buf = BytesMut::with_capacity( 16 );
		let mut rng = rand::thread_rng();

		// u128 doesn't work in wasm and serde is being a pain, so 2 u64
		//
		buf.put_u64_le( rng.gen::<u64>() );
		buf.put_u64_le( rng.gen::<u64>() );

		Self { bytes: buf.freeze() }
	}


	/// Seed the UniqueID. The data will be hashed with xxhash. The UniqueID is 128 bits and currently
	/// xxhash only supports hashing to 64bit output. We take 2 seeds, one for each 64bit hash.
	/// Parameter high will be the most significant bytes in the output in little endian (msb is highest offset).
	///
	/// An identical input here should always give an identical UniqueID.
	//
	pub(crate) fn from_seed( high: &[u8], low: &[u8] ) -> Self
	{
		let mut h = XxHash64::default();
		let mut l = XxHash64::default();

		h.write( high );
		l.write( low  );

		// FIXME: keep an eye on xxh3 support.
		//
		let mut buf = BytesMut::with_capacity( 128 );

		buf.put_u64_le( l.finish() );
		buf.put_u64_le( h.finish() );

		Self { bytes: buf.freeze() }
	}


	/// And empty UniqueID. Can be used to signify the abscence of an id, would usually be all
	/// zero bytes.
	//
	pub(crate) fn null() -> Self
	{
		let mut data = BytesMut::with_capacity( 16 );
		data.put_u64_le( 0 );
		data.put_u64_le( 0 );

		Self { bytes: data.freeze() }
	}


	/// And full UniqueID. Reserved for use by thespis_remote, would usually be all
	/// one bytes.
	/// u128 still isn't well supported everywhere (WASM), so use 2 u64;
	//
	pub(crate) fn full() -> Self
	{
		let mut data = BytesMut::with_capacity( 16 );
		data.put_u64_le( u64::max_value() );
		data.put_u64_le( u64::max_value() );

		Self { bytes: data.freeze() }
	}


	/// Predicate for null values (all bytes are 0).
	//
	pub(crate) fn is_null( &self ) -> bool
	{
		self.bytes.iter().all( |b| *b == 0 )
	}


	/// Predicate for null values (all bytes are 0).
	//
	pub(crate) fn is_full( &self ) -> bool
	{
		self.bytes.iter().all( |b| *b == 0xff )
	}
}



/// Internally is also represented as Bytes, so you just get a copy.
//
impl Into< Bytes > for UniqueID
{
	fn into( self ) -> Bytes
	{
		self.bytes
	}
}


/// The object will just keep the bytes as internal representation, no copies will be made
//
impl From< Bytes > for UniqueID
{
	fn from( bytes: Bytes ) -> Self
	{
		Self { bytes }
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
		write!( f, "0x" )?;

		fmt::LowerHex::fmt( &self, f )
	}
}


impl fmt::LowerHex for UniqueID
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		// Reverse because we are in little endian, and hexadecimal numbers are commonly written in big-endian.
		//
		for byte in self.bytes.iter().rev()
		{
			write!( f, "{:02x}", &byte )?
		}

		Ok(())
	}
}


impl Serialize for UniqueID
{
	fn serialize<S: serde::Serializer>( &self, serializer: S ) -> Result<S::Ok, S::Error>
	{
		serde_bytes::serialize( &self.bytes[..], serializer )
	}
}


impl<'de> Deserialize<'de> for UniqueID
{
	fn deserialize<D: serde::Deserializer<'de>>( deserializer: D ) -> Result<Self, D::Error>
	{
		// converting from a slice, bytes turns it into a vec anyways, so this
		// avoids the lifetime mess for free.
		//
		serde_bytes::deserialize( deserializer ).map( |bytes: Vec<u8>|
		{
			Self { bytes: bytes.into() }
		})
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
		let sid  = UniqueID::from_seed( b"namespace", b"Typename" );
		let sid2 = UniqueID::from_seed( b"namespace", b"Typename" );

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
		let bytes: Bytes = vec![ 0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, ].into();
		let sid = UniqueID::try_from( bytes ).unwrap();

		assert_eq!( "0x0f0e0d0c0b0a09080706050403020100", &format!( "{:?}", sid ) );
	}
}


