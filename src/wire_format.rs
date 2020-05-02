//! Implementation of the MultiService wire format.
//
mod unique_id  ;
mod conn_id    ;
mod service_id ;
mod wire_type  ;

pub use
{
	service_id :: * ,
	conn_id    :: * ,
};

pub(crate) use wire_type::WireType;


#[ cfg(any( feature = "futures_codec", feature = "tokio_codec" )) ] pub mod codec   ;
#[ cfg(any( feature = "futures_codec", feature = "tokio_codec" )) ] pub use codec::*;

use crate::{ import::*, ThesRemoteErr, ErrorContext };

const HEADER_LEN: usize = 32;

/// A multi service message.
///
/// The message format is as follows:
///
/// The format is little endian.
///
/// length  : the length in bytes of the payload
/// sid     : user chosen sid for the service
/// connID  : in case of a call, which requires a response, a unique random number
///           in case of a send, which does not require response, zero
/// message : the request message serialized with the specified codec
///
/// ```text
/// u64 length + payload --------------------------------------------|
///              16 bytes sid | 16 bytes connID | serialized message |
///              u128 LE      | u128 LE         | variable           |
/// ------------------------------------------------------------------
/// ```
///
/// As soon as a codec determines from the length field that the entire message is read,
/// they can create a Multiservice from the bytes. In general creating a Multiservice
/// object should not perform a copy of the serialized message. It just provides a window
/// to look into the multiservice message to figure out if:
/// - the service sid is meant to be delivered to an actor on the current process or is to be relayed.
/// - if unknown, if there is a non-zero connID, in which case an error is returned to the sender
/// - if it is meant for the current process, deserialize it with `encoding` and send it to the correct actor.
///
/// The algorithm for receiving messages should interprete the message like this:
///
/// ```text
/// - msg for a local actor -> service sid is in our table
///   - send                -> no connID
///   - call                -> connID
///
/// - a send/call for an actor we don't know -> sid unknown: respond with error
///
/// - a response to a call    -> valid connID
///   - when the call was made, we gave a onshot-channel receiver to the caller,
///     look it up in our open connections table and put the response in there.
///
/// - an error message (eg. deserialization failed on the remote) -> service sid null.
///
///
///   -> log the error, we currently don't have a way to pass an error to the caller.
///      We should provide a mechanism for the application to handle the errors.
///      The deserialized message shoudl be a string error.
///
/// Possible errors:
/// - Destination unknown (since an address is like a capability,
///   we don't distinguish between not permitted and unknown)
/// - Fail to deserialize message
//
#[ derive( Debug, Clone, PartialEq, Eq ) ]
//
pub struct WireFormat
{
	bytes: Bytes
}


impl Message for WireFormat
{
	type Return = Result<(), ThesRemoteErr>;
}



// All the methods here can panic. We should make sure that bytes is always big enough,
// because bytes.slice panics if it's to small. Same for bytes.put.
//
#[ allow(clippy::len_without_is_empty) ]
//
impl WireFormat
{
	pub fn create( service: ServiceID, conn_id: ConnID, mesg: Bytes ) -> Self
	{
		let mut bytes = BytesMut::with_capacity( HEADER_LEN + mesg.len() );

		bytes.put( Into::<Bytes>::into( service ) );
		bytes.put( Into::<Bytes>::into( conn_id ) );
		bytes.put( mesg                           );

		Self { bytes: bytes.freeze() }
	}


	/// The service id of this message. When coming in over the wire, this identifies
	/// which service you are calling. A ServiceID should be unique for a given service.
	/// The reference implementation combines a unique type id with a namespace so that
	/// several processes can accept the same type of service under a unique name each.
	//
	pub fn service ( &self ) -> ServiceID
	{
		ServiceID::from( self.bytes.slice( 0..16 ) )
	}


	/// The connection id. This is used to match responses to outgoing calls.
	//
	pub fn conn_id( &self ) -> ConnID
	{
		ConnID::from( self.bytes.slice( 16..32 ) )
	}


	/// The serialized payload message.
	//
	pub fn mesg( &self ) -> Bytes
	{
		self.bytes.slice( HEADER_LEN.. )
	}

	/// The total length of the Multiservice in Bytes (header+payload)
	//
	pub fn len( &self ) -> usize
	{
		self.bytes.len()
	}

	/// Find out what kind of message this is.
	//
	pub(crate) fn kind( &self ) -> WireType
	{
		match self.service()
		{
			x if x.is_null() => WireType::ConnectionError ,
			x if x.is_full() => WireType::CallResponse    ,

			_ =>
			{
				match self.conn_id()
				{
					x if x.is_null() => WireType::IncomingSend ,
					_                => WireType::IncomingCall ,
				}
			}
		}
	}
}



impl Into< Bytes > for WireFormat
{
	fn into( self ) -> Bytes
	{
		self.bytes
	}
}



impl TryFrom< Bytes > for WireFormat
{
	type Error = ThesRemoteErr;

	fn try_from( bytes: Bytes ) -> Result< Self, ThesRemoteErr >
	{
		// at least verify we have enough bytes
		// We allow an empty message. In principle I suppose a zero sized type could be
		// serialized to nothing by serde. Haven't checked though.
		//
		if bytes.len() < HEADER_LEN
		{
			return Err( ThesRemoteErr::DeserializeWireFormat{ ctx: ErrorContext
			{
				context  : "WireFormat: not enough bytes even for the header.".to_string().into() ,
				sid      : None                                                                   ,
				cid      : None                                                                   ,
				peer_id  : None                                                                   ,
				peer_name: None                                                                   ,

			}});
		}

		Ok( Self { bytes } )
	}
}







#[ cfg(test) ]
//
mod tests
{
	// Tests:
	//
	// 1. try_from bytes, try something to small
	//

	use super::{ * };


	#[test]
	//
	fn tryfrom_bytes_to_small()
	{
		// The minimum size of the header + 1 byte payload is 33
		//
		let buf = Bytes::from( vec![5;HEADER_LEN-1] );

		match WireFormat::try_from( buf )
		{
			Ok (_) => panic!( "WireFormat::try_from( Bytes ) should fail for data shorter than header" ),

			Err(e) => match e
			{
				ThesRemoteErr::DeserializeWireFormat{..} => {}
				_                                        => panic!( "Wrong error type (should be DeserializeWireFormat): {:?}", e ),
			}
		}
	}
}
