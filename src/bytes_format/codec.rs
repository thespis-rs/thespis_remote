//! The codec to frame the connection with our wire format.
//
use crate::{ import::*, BytesFormat, wire_format::* };
use super::HEADER_LEN;

/// The tokio/futures codec to frame AsyncRead/Write streams.
//
#[ derive( Debug ) ]
//
pub struct ThesCodec
{
	max_size: usize, // in bytes
}


impl ThesCodec
{
	/// Create a new codec, with the max length of a single message in bytes. Note that this includes the
	/// header of the BytesFormat. For [`thespis_remote_impl::BytesFormat`] the header is 36 bytes.
	///
	/// Setting a value to high might lead to more memory usage and could enable OOM/DDOS attacks.
	/// Note that the `max_size` is the max size of your client message, but actual data sent over the
	/// wire will have 32 more bytes from the BytesFormat header + 8 bytes for a length field from the codec.
	//
	pub fn new( max_size: usize ) -> Self
	{
		Self{ max_size }
	}


	// TODO: zero copy encoding. Currently we copy bytes in the output buffer.
	// In principle we would like to not have to serialize the inner message
	// before having access to this buffer.
	//
	fn encode_impl( &mut self, item: BytesFormat, buf: &mut BytesMut ) -> Result<(), WireErr>
	{
		let payload_len = item.len() - HEADER_LEN;

		// respect the max_size
		//
		if payload_len > self.max_size
		{
			return Err( WireErr::MessageSizeExceeded
			{
				context : "BytesFormat Codec encoder".to_string() ,
				size    : payload_len                            ,
				max_size: self.max_size                          ,
			})
		}


		let len = item.len() + 8;
		buf.reserve( len );

		buf.put_u64_le( len as u64 );
		buf.put( Into::<Bytes>::into( item ) );

		Ok(())
	}


	fn decode_impl( &mut self, buf: &mut BytesMut ) -> Result< Option<BytesFormat>, WireErr >
	{
		// trace!( "Decoding incoming message: {:?}", &buf );

		// Minimum length of an empty message is the u64 indicating the length + the header (sid/connid)
		//
		if buf.len() < 8 + HEADER_LEN  { return Ok( None ) }


		// parse the first 8 bytes to find out the total length of the message
		//
		let mut tmp     = Bytes::from( buf[..8].to_vec() ) ;
		let mut len     = tmp.get_u64_le() as usize        ;
		let payload_len = len - HEADER_LEN - 8             ;

		// respect the max_size
		//
		if payload_len > self.max_size
		{
			return Err( WireErr::MessageSizeExceeded
			{
				context : "BytesFormat Codec decoder".to_string() ,
				size    : payload_len                            ,
				max_size: self.max_size                          ,
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
		Ok( Some( BytesFormat::try_from( buf.split_to( len ).freeze() )? ) )
	}
}



#[ cfg( feature = "futures_codec" ) ]
//
impl FutDecoder for ThesCodec
{
	type Item  = BytesFormat ;
	type Error = WireErr    ;

	fn decode( &mut self, buf: &mut BytesMut ) -> Result< Option<Self::Item>, Self::Error >
	{
		self.decode_impl( buf )
	}
}


#[ cfg( feature = "futures_codec" ) ]
//
impl FutEncoder for ThesCodec
{
	type Item  = BytesFormat ;
	type Error = WireErr    ;

	fn encode( &mut self, item: Self::Item, buf: &mut BytesMut ) -> Result<(), Self::Error>
	{
		self.encode_impl( item, buf )
	}
}


#[ cfg( feature = "tokio_codec" ) ]
//
impl TokioDecoder for ThesCodec
{
	type Item  = BytesFormat ;
	type Error = WireErr    ;

	fn decode( &mut self, buf: &mut BytesMut ) -> Result< Option<Self::Item>, Self::Error >
	{
		self.decode_impl( buf )
	}
}



#[ cfg( feature = "tokio_codec" ) ]
//
impl TokioEncoder for ThesCodec
{
	type Item  = BytesFormat ;
	type Error = WireErr    ;

	fn encode( &mut self, item: Self::Item, buf: &mut BytesMut ) -> Result<(), Self::Error>
	{
		self.encode_impl( item, buf )
	}
}


#[ cfg( test ) ]
//
macro_rules! test_codec
{
	(
		Encoder: $Encoder: ident,
		Decoder: $Decoder: ident$(,)?

	) =>

	{
		// Tests:
		//
		// 1. A valid BytesFormat, encoded + decoded should be identical to original.
		// 2. Send like 2 and a half full objects, test that 2 correctly come out, and there is the
		//    exact amount of bytes left in the buffer for the other half.
		// 3. test max_size. Verify that a max_size exactly the size of the message passed (tested in test full),
		//    and test the error (tested in test max_size)
		// 4. calling decode on short or empty buffer should return Ok(None)
		//
		fn empty_data() -> BytesFormat
		{
			let mut buf = BytesMut::with_capacity( 1 );
			buf.put( &[0u8;1][..] );

			let m = BytesFormat::create( ServiceID::from_seed( b"codec_tests", b"Empty Message" ), ConnID::random(), buf.freeze() );

			m
		}

		fn full_data() -> BytesFormat
		{
			let mut buf = BytesMut::with_capacity( 5 );
			buf.put( "hello".as_bytes() );

			BytesFormat::create( ServiceID::from_seed( b"codec_tests", b"Full Message" ), ConnID::random(), buf.freeze() )
		}


		#[test]
		//
		fn empty()
		{
			let mut codec = ThesCodec::new( 1024 );
			let mut buf   = BytesMut::new();
			let     data  = empty_data();
			let     data2 = data.clone();

			$Encoder::encode( &mut codec, data, &mut buf ).expect( "Encoding empty" );

			assert_eq!
			(
				data2,

				$Decoder::decode( &mut codec, &mut buf )

					.expect( "No errors should occur"                      )
					.expect( "There should be some data (eg. Not Ok(None)" )
			);
		}


		#[test]
		//
		fn full()
		{
			// Set max_size exactly, the full data is a 5 byte string
			//
			let mut codec = ThesCodec::new(5);
			let mut buf   = BytesMut::new();
			let     data  = full_data();
			let     data2 = data.clone();

			$Encoder::encode( &mut codec, data, &mut buf ).expect( "Encoding empty" );

			assert_eq!
			(
				data2,

				$Decoder::decode( &mut codec, &mut buf )

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

			$Encoder::encode( &mut codec, empty, &mut buf ).expect( "Encoding empty" ); // 41 bytes
			$Encoder::encode( &mut codec, full , &mut buf ).expect( "Encoding full"  ); // 45 bytes
			$Encoder::encode( &mut codec, full2, &mut buf ).expect( "Encoding full"  ); // 45 bytes

			// total is 131
			//
			assert_eq!( empty2, $Decoder::decode( &mut codec, &mut buf ).expect( "Decode empty" ).expect( "Not None" ) );
			assert_eq!(  full3, $Decoder::decode( &mut codec, &mut buf ).expect( "Decode empty" ).expect( "Not None" ) );

			assert_eq!( 45, buf.len() ); // there should be exactly 48 bytes sitting there waiting for the last.
		}


		#[test]
		//
		fn max_size()
		{
			// Verify that max_size is respected.
			//
			let mut codec = ThesCodec::new(4);
			let mut buf   = BytesMut::new();
			let     data  = full_data();

			let res = $Encoder::encode( &mut codec, data, &mut buf );

			assert_eq!
			(
				res.expect_err( "exceeding max size should give an error" ),

				WireErr::MessageSizeExceeded
				{
					context : "BytesFormat Codec encoder".to_string() ,
					size    : 5                                      ,
					max_size: 4                                      ,
				}
			);
		}


		#[test]
		//
		fn decode_empty()
		{
			// Verify that max_size is respected.
			//
			let mut codec = ThesCodec::new(4);
			let mut buf   = BytesMut::new();

			let res = $Decoder::decode( &mut codec, &mut buf );

			assert_eq!( res, Ok(None) );
		}


		#[test]
		//
		fn decode_short()
		{
			// Verify that max_size is respected.
			//
			let mut codec = ThesCodec::new(4);
			let mut buf   = BytesMut::new();

			buf.put_u8( 3 );

			let res = $Decoder::decode( &mut codec, &mut buf );

			assert_eq!( res, Ok(None) );
		}
	}
}


#[ cfg(all( test, feature = "futures_codec" )) ]
//
mod tests_futures
{
	use crate :: { *            } ;
	use super :: { *, assert_eq } ;

	test_codec!( Encoder: FutEncoder, Decoder: FutDecoder );
}


#[ cfg(all( test, feature = "tokio_codec" )) ]
//
mod tests_tokio
{
	use crate :: { *            } ;
	use super :: { *, assert_eq } ;

	test_codec!( Encoder: TokioEncoder, Decoder: TokioDecoder );
}



