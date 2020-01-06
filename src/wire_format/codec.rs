//! The codec to frame the connection with our wire format.
//
use crate::{ import::*, WireFormat, ThesRemoteErr };
use super::HEADER_LEN;

/// The tokio codec to frame AsyncRead/Write streams.
/// TODO: test max_length
//
#[ derive( Debug ) ]
//
pub struct ThesCodec
{
	max_length: usize  , // in bytes

}


impl ThesCodec
{
	/// Create a new codec, with the max length of a single message in bytes. Note that this includes the
	/// header of the wireformat. For [`thespis_remote_impl::WireFormat`] the header is 36 bytes.
	///
	/// Setting a value to high might lead to more memory usage and could enable OOM/DDOS attacks.
	//
	pub fn new( max_length: usize ) -> Self
	{
		Self{ max_length }
	}


	fn encode_impl( &mut self, item: WireFormat, buf: &mut BytesMut ) -> Result<(), ThesRemoteErr>
	{
		// respect the max_length
		//
		if item.len() > self.max_length
		{
			return Err( ThesRemoteErr::MessageSizeExceeded
			(
				format!( "Tokio Codec Encoder: max_length={:?} bytes, message={:?} bytes", self.max_length, item.len() )

			).into() )
		}


		let len = item.len() + 8;
		buf.reserve( len );

		let mut wtr = vec![];
		wtr.write_u64::<LittleEndian>( len as u64 ).expect( "Tokio codec encode: Write u64 to vec" );

		buf.put( wtr.as_ref()                );
		buf.put( Into::<Bytes>::into( item ) );

		Ok(())
	}


	fn decode_impl( &mut self, buf: &mut BytesMut ) -> Result< Option<WireFormat>, ThesRemoteErr >
	{
		trace!( "Decoding incoming message: {:?}", &buf );

		// Minimum length of an empty message is the u64 indicating the length + the header (sid/connid)
		//
		if buf.len() < 8 + HEADER_LEN  { return Ok( None ) }


		// parse the first 8 bytes to find out the total length of the message
		//
		let mut len = buf[..8]

			.as_ref()
			.read_u64::<LittleEndian>()
			.map_err( |_| ThesRemoteErr::Deserialize( "Tokio codec: Length".to_string() ))?

			as usize
		;


		// respect the max_length
		// TODO: does max length include the HEADER? Document and make consistent with Encoder.
		//
		if len > self.max_length
		{
			return Err( ThesRemoteErr::MessageSizeExceeded
			(
				format!( "Tokio Codec Decoder: max_length={:?} bytes, message={:?} bytes", self.max_length, len )

			).into())
		}

		// Message hasn't completely arrived yet
		//
		if buf.len() < len { return Ok( None ) }


		// We have an entire message.
		// Remove the length header, we won't need it anymore
		//
		buf.advance( 8 );
		len -= 8;

		// Consume the message
		//
		let buf = buf.split_to( len );

		// Convert
		//
		Ok( Some( WireFormat::try_from( buf.freeze() )? ) )
	}
}


#[ cfg( feature = "futures_codec" ) ]
//
impl FutDecoder for ThesCodec
{
	type Item  = WireFormat ;
	type Error = ThesRemoteErr    ;

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
	type Item  = WireFormat ;
	type Error = ThesRemoteErr    ;

	fn encode( &mut self, item: Self::Item, buf: &mut BytesMut ) -> Result<(), Self::Error>
	{
		self.encode_impl( item, buf )
	}
}


#[ cfg( feature = "tokio_codec" ) ]
//
impl TokioDecoder for ThesCodec
{
	type Item  = WireFormat ;
	type Error = ThesRemoteErr    ;

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
	type Item  = WireFormat ;
	type Error = ThesRemoteErr    ;

	fn encode( &mut self, item: Self::Item, buf: &mut BytesMut ) -> Result<(), Self::Error>
	{
		self.encode_impl( item, buf )
	}
}


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
	use super::{ *, assert_eq };



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

		<ThesCodec as FutEncoder>::encode( &mut codec, data, &mut buf ).expect( "Encoding empty" );

		assert_eq!
		(
			data2,

			<ThesCodec as FutDecoder>::decode( &mut codec, &mut buf )

				.expect( "No errors should occur"                      )
				.expect( "There should be some data (eg. Not Ok(None)" )
		);
	}


	#[test]
	//
	fn full()
	{
		let mut codec = ThesCodec::new(1024);
		let mut buf   = BytesMut::new();
		let     data  = full_data();
		let     data2 = data.clone();

		<ThesCodec as FutEncoder>::encode( &mut codec, data, &mut buf ).expect( "Encoding empty" );

		assert_eq!
		(
			data2,

			<ThesCodec as FutDecoder>::decode( &mut codec, &mut buf )

				.expect( "No errors should occur"                      )
				.expect( "There should be some data (eg. Not Ok(None)" )
		);
	}


	#[test]
	//
	fn partials()
	{
		let mut codec: ThesCodec = ThesCodec ::new(1024);

		let mut buf = BytesMut::new();

		let     empty  = empty_data();
		let     full   = full_data ();

		let     empty2 = empty.clone();
		let     full2  = full .clone();
		let     full3  = full .clone();

		<ThesCodec as FutEncoder>::encode( &mut codec, empty, &mut buf ).expect( "Encoding empty" ); // 41 bytes
		<ThesCodec as FutEncoder>::encode( &mut codec, full , &mut buf ).expect( "Encoding full"  ); // 45 bytes
		<ThesCodec as FutEncoder>::encode( &mut codec, full2, &mut buf ).expect( "Encoding full"  ); // 45 bytes

		// total is 131
		//
		assert_eq!( empty2, <ThesCodec as FutDecoder>::decode( &mut codec, &mut buf )

				.expect( "Decode empty" ).expect( "Not None" ) );

		assert_eq!(  full3, <ThesCodec as FutDecoder>::decode( &mut codec, &mut buf )

				.expect( "Decode empty" ).expect( "Not None" ) );

		assert_eq!( 45, buf.len() ); // there should be exactly 48 bytes sitting there waiting for the last.
	}
}
