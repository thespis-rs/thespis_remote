//! The codec to frame the connection with our wire format.
//
use crate::{ import::*, WireFormat, ThesRemoteErr };
use super::HEADER_LEN;

/// The tokio/futures codec to frame AsyncRead/Write streams.
//
#[ derive( Debug ) ]
//
pub struct ThesCodec
{
	max_length: usize, // in bytes
}


impl ThesCodec
{
	/// Create a new codec, with the max length of a single message in bytes. Note that this includes the
	/// header of the wireformat. For [`thespis_remote_impl::WireFormat`] the header is 36 bytes.
	///
	/// Setting a value to high might lead to more memory usage and could enable OOM/DDOS attacks.
	/// Note that the `max_length` is the max size of your client message, but actual data sent over the
	/// wire will have 32 more bytes from the WireFormat header + 8 bytes for a length field from the codec.
	//
	pub fn new( max_length: usize ) -> Self
	{
		Self{ max_length }
	}


	fn encode_impl( &mut self, item: WireFormat, buf: &mut BytesMut ) -> Result<(), ThesRemoteErr>
	{
		let payload_len = item.len() - HEADER_LEN;

		// respect the max_length
		//
		if payload_len > self.max_length
		{
			return Err( ThesRemoteErr::MessageSizeExceeded
			{
				context : "WireFormat Codec encoder".to_string() ,
				size    : payload_len                            ,
				max_size: self.max_length                        ,
			})
		}


		let len = item.len() + 8;
		buf.reserve( len );

		buf.put_u64_le( len as u64 );
		buf.put( Into::<Bytes>::into( item ) );

		Ok(())
	}


	fn decode_impl( &mut self, buf: &mut BytesMut ) -> Result< Option<WireFormat>, ThesRemoteErr >
	{
		// trace!( "Decoding incoming message: {:?}", &buf );

		// Minimum length of an empty message is the u64 indicating the length + the header (sid/connid)
		//
		if buf.len() < 8 + HEADER_LEN  { return Ok( None ) }


		// parse the first 8 bytes to find out the total length of the message
		//
		let mut tmp     = Bytes::from( buf[..8].to_vec() );
		let mut len     = tmp.get_u64_le() as usize;
		let payload_len = len - HEADER_LEN - 8;

		// respect the max_length
		//
		if payload_len > self.max_length
		{
			return Err( ThesRemoteErr::MessageSizeExceeded
			{
				context : "WireFormat Codec decoder".to_string() ,
				size    : payload_len                            ,
				max_size: self.max_length                        ,
			})
		}

		// Message hasn't completely arrived yet
		//
		if buf.len() < len { return Ok( None ) }


		// We have an entire message.
		// Remove the length header, we won't need it anymore
		//
		buf.advance( 8 );
		len -= 8;

		// Consume the message & Convert
		//
		Ok( Some( WireFormat::try_from( buf.split_to( len ).freeze() )? ) )
	}
}



#[ cfg( feature = "futures_codec" ) ]
//
impl FutDecoder for ThesCodec
{
	type Item  = WireFormat    ;
	type Error = ThesRemoteErr ;

	fn decode( &mut self, buf: &mut BytesMut ) -> Result< Option<Self::Item>, Self::Error >
	{
		self.decode_impl( buf )
	}
}


// TODO: zero copy encoding. Currently we copy bytes in the output buffer.
// In principle we would like to not have to serialize the inner message
// before having access to this buffer.
//
#[ cfg( feature = "futures_codec" ) ]
//
impl FutEncoder for ThesCodec
{
	type Item  = WireFormat    ;
	type Error = ThesRemoteErr ;

	fn encode( &mut self, item: Self::Item, buf: &mut BytesMut ) -> Result<(), Self::Error>
	{
		self.encode_impl( item, buf )
	}
}


#[ cfg( feature = "tokio_codec" ) ]
//
impl TokioDecoder for ThesCodec
{
	type Item  = WireFormat    ;
	type Error = ThesRemoteErr ;

	fn decode( &mut self, buf: &mut BytesMut ) -> Result< Option<Self::Item>, Self::Error >
	{
		self.decode_impl( buf )
	}
}


// TODO: zero copy encoding. Currently we copy bytes in the output buffer.
// In principle we would like to not have to serialize the inner message
// before having access to this buffer.
//
#[ cfg( feature = "tokio_codec" ) ]
//
impl TokioEncoder for ThesCodec
{
	type Item  = WireFormat    ;
	type Error = ThesRemoteErr ;

	fn encode( &mut self, item: Self::Item, buf: &mut BytesMut ) -> Result<(), Self::Error>
	{
		self.encode_impl( item, buf )
	}
}


// TODO: test tokio
//
#[ cfg(all( test, feature = "futures_codec" )) ]
//
mod tests
{
	// Tests:
	//
	// 1. A valid WireFormat, encoded + decoded should be identical to original.
	// 2. Send like 2 and a half full objects, test that 2 correctly come out, and there is the
	//    exact amount of bytes left in the buffer for the other half.
	// 3. TODO: send invalid data (not enough bytes to make a full multiservice header...)
	//
	use crate::{ * };
	use super::{ *, assert_eq, assert_matches };



	fn empty_data() -> WireFormat
	{
		let mut buf = BytesMut::with_capacity( 1 );
		buf.put( &[0u8;1][..] );

		let m = WireFormat::create( ServiceID::from_seed( b"codec_tests", b"Empty Message" ), ConnID::random(), buf.freeze() );

		m
	}

	fn full_data() -> WireFormat
	{
		let mut buf = BytesMut::with_capacity( 5 );
		buf.put( "hello".as_bytes() );

		WireFormat::create( ServiceID::from_seed( b"codec_tests", b"Full Message" ), ConnID::random(), buf.freeze() )
	}


	#[test]
	//
	fn empty()
	{
		let mut codec = ThesCodec::new( 1024 );
		let mut buf   = BytesMut::new();
		let     data  = empty_data();
		let     data2 = data.clone();

		FutEncoder::encode( &mut codec, data, &mut buf ).expect( "Encoding empty" );

		assert_eq!
		(
			data2,

			FutDecoder::decode( &mut codec, &mut buf )

				.expect( "No errors should occur"                      )
				.expect( "There should be some data (eg. Not Ok(None)" )
		);
	}


	#[test]
	//
	fn full()
	{
		// Set max_length exactly, the full data is a 5 byte string
		//
		let mut codec = ThesCodec::new(5);
		let mut buf   = BytesMut::new();
		let     data  = full_data();
		let     data2 = data.clone();

		FutEncoder::encode( &mut codec, data, &mut buf ).expect( "Encoding empty" );

		assert_eq!
		(
			data2,

			FutDecoder::decode( &mut codec, &mut buf )

				.expect( "No errors should occur"                      )
				.expect( "There should be some data (eg. Not Ok(None)" )
		);
	}


	#[test]
	//
	fn partials()
	{
		let mut codec = ThesCodec::new(1024);
		let mut buf   = BytesMut::new();

		let     empty  = empty_data();
		let     full   = full_data ();

		let     empty2 = empty.clone();
		let     full2  = full .clone();
		let     full3  = full .clone();

		FutEncoder::encode( &mut codec, empty, &mut buf ).expect( "Encoding empty" ); // 41 bytes
		FutEncoder::encode( &mut codec, full , &mut buf ).expect( "Encoding full"  ); // 45 bytes
		FutEncoder::encode( &mut codec, full2, &mut buf ).expect( "Encoding full"  ); // 45 bytes

		// total is 131
		//
		assert_eq!( empty2, FutDecoder::decode( &mut codec, &mut buf ).expect( "Decode empty" ).expect( "Not None" ) );
		assert_eq!(  full3, FutDecoder::decode( &mut codec, &mut buf ).expect( "Decode empty" ).expect( "Not None" ) );

		assert_eq!( 45, buf.len() ); // there should be exactly 48 bytes sitting there waiting for the last.
	}


	#[test]
	//
	fn max_length()
	{
		// Verify that max_length is respected.
		//
		let mut codec = ThesCodec::new(4);
		let mut buf   = BytesMut::new();
		let     data  = full_data();

		let res = FutEncoder::encode( &mut codec, data, &mut buf );

		assert_matches!
		(
			res,

			Err( ThesRemoteErr::MessageSizeExceeded{ context, size, max_size } ) if

				   context  == "WireFormat Codec encoder".to_string()
				&& size     == 5
				&& max_size == 4
		);
	}
}
