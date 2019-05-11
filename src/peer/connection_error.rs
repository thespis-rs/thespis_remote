use crate :: { import::* };

/// All errors that can happen when receiving messages over the wire
/// These will be broadcast to observers, so you can act upon them if necessary.
/// This type is a bit special as an error type, as here it is not being returned
/// in [`Result`] since the caller is remote. It is generally returned serialized
/// over the wire and broadcast to observers.
//
#[ derive( Debug, Clone, PartialEq, Serialize, Deserialize ) ]
//
pub enum ConnectionError
{
	Deserialize                   ,
	Serialize                     ,
	InternalServerError           ,
	LostRelayBeforeCall           , // the remote should know which service thanks to connID
	LostRelayBeforeResponse       , // the remote should know which service thanks to connID
	LostRelayBeforeSend(Vec<u8>)  , // here there is no ConnID, so we put the sid back in there.
	UnknownService(Vec<u8>)       ,
}
