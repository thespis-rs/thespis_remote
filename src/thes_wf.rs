
use
{
	crate     :: { import::*, PeerErr, wire_format::*  } ,
	byteorder :: { ReadBytesExt, LittleEndian          } ,
	std       :: { io::Seek                            } ,
};


mod encoder;
mod decoder;

pub use encoder::*;
pub use decoder::*;

const LEN_LEN: usize = 8; // u64
const LEN_SID: usize = 8; // u64
const LEN_CID: usize = 8; // u64


const IDX_LEN: usize = 0;
const IDX_SID: usize = LEN_LEN;
const IDX_CID: usize = IDX_SID + LEN_SID;
const IDX_MSG: usize = IDX_CID + LEN_CID;

const LEN_HEADER: usize = IDX_MSG;



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
/// u64 length + payload ------------------------------------------|
///              8 bytes sid | 8 bytes connID | serialized message |
///              u64 LE      | u64 LE         | variable           |
/// ----------------------------------------------------------------
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
pub struct ThesWF
{
	data: io::Cursor< Vec<u8> >,
}



impl Message for ThesWF
{
	type Return = Result<(), PeerErr>;
}



// All the methods here can panic. We should make sure that bytes is always big enough,
// because bytes.slice panics if it's to small. Same for bytes.put.
//
impl WireFormat for ThesWF
{

	/// The service id of this message. When coming in over the wire, this identifies
	/// which service you are calling. A ServiceID should be unique for a given service.
	/// The reference implementation combines a unique type id with a namespace so that
	/// several processes can accept the same type of service under a unique name each.
	//
	fn service ( &self ) -> ServiceID
	{
		// TODO: is this the most efficient way?
		//
		self.data.get_ref()[ IDX_SID..IDX_SID+LEN_SID ].as_ref().read_u64::<LittleEndian>().unwrap().into()
	}


	/// The connection id. This is used to match responses to outgoing calls.
	//
	fn conn_id( &self ) -> ConnID
	{
		self.data.get_ref()[ IDX_CID..IDX_CID+LEN_CID ].as_ref().read_u64::<LittleEndian>().unwrap().into()
	}


	/// The serialized payload message.
	//
	fn mesg( &self ) -> &[u8]
	{
		&self.data.get_ref()[ IDX_MSG.. ]
	}

	/// The total length of the ThesWF in bytes (header+payload)
	//
	fn len( &self ) -> u64
	{
		self.data.get_ref()[ IDX_LEN..IDX_LEN+LEN_LEN ].as_ref().read_u64::<LittleEndian>().unwrap()
	}

	/// Make sure there is enough room for the serialized payload to avoid frequent re-allocation.
	//
	fn reserve( &mut self, additional: usize )
	{
		self.data.get_mut().reserve( additional )
	}
}



impl io::Write for ThesWF
{
	fn write( &mut self, buf: &[u8] ) -> io::Result<usize>
	{
		self.data.seek( io::SeekFrom::End(0) )?;
		self.data.write( buf )
	}

	fn flush( &mut self ) -> io::Result<()>
	{
		Ok(())
	}
}



impl Default for ThesWF
{
	fn default() -> Self
	{
		Self { data: io::Cursor::new( Vec::new() ) }
	}
}



impl TryFrom< Vec<u8> > for ThesWF
{
	type Error = WireErr;

	fn try_from( data: Vec<u8> ) -> Result< Self, WireErr >
	{
		// at least verify we have enough bytes
		// We allow an empty message. In principle I suppose a zero sized type could be
		// serialized to nothing by serde. Haven't checked though.
		//
		if data.len() < LEN_HEADER
		{
			return Err( WireErr::Deserialize{ context: "ThesWF: not enough bytes even for the header.".to_string() } );
		}

		Ok( Self { data: io::Cursor::new(data) } )
	}
}







// #[ cfg(test) ]
// //
// mod tests
// {
// 	// Tests:
// 	//
// 	// 1. try_from bytes, try something to small
// 	//

// 	use super::{ * };


// 	#[test]
// 	//
// 	fn tryfrom_bytes_to_small()
// 	{
// 		// The minimum size of the header + 1 byte payload is 33
// 		//
// 		let buf = Bytes::from( vec![5;HEADER_LEN-1] );

// 		match ThesWF::try_from( buf )
// 		{
// 			Ok (_) => panic!( "ThesWF::try_from( Bytes ) should fail for data shorter than header" ),

// 			Err(e) => match e
// 			{
// 				WireErr::Deserialize{..} => {}
// 				_                        => panic!( "Wrong error type (should be DeserializeWireFormat): {:?}", e ),
// 			}
// 		}
// 	}
// }
