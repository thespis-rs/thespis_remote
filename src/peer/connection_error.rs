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
	/// An error deserializing the incoming message. This means the stream might be corrupt,
	/// so the connection will be closed.
	//
	Deserialize,

	/// An error happend when trying to serialize a response.
	//
	Serialize,

	/// Your request could not be processed. This might mean spawning a future failed,
	/// downcasting a Receiver failed or other errors that are clearly not the fault
	/// of the remote peer.
	//
	InternalServerError,

	/// Warn our remote peer that we are no longer providing this service.
	/// The data is the sid for which we stop providing. This can happen if a relay goes down.
	/// This means that any further calls to the peer for this service will return
	/// ConnectionError::ServiceUnknown
	//
	ServiceGone(Vec<u8>),

	/// Sending out your message on the connection to the relayed peer failed. If this is
	/// a permanent failure, eg. ConnectionClosed, you should also get RelayGone errors
	/// for all the services that are lost.
	//
	FailedToRelay(Vec<u8>),

	/// Whilst waiting for the response to your call, the connection to the relay was lost.
	/// You should also get RelayGone errors  for all the services that are lost.
	//
	LostRelayBeforeResponse,

	/// We don't provide this service. Data is sid.
	//
	UnknownService(Vec<u8>),

	/// The codec is not valid for the operation.
	/// The data is the codec passsed in, in serialized form.
	//
	UnsupportedCodec(Vec<u8>)
}
