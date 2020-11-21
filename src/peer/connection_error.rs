use crate :: { import::*, ServiceID, ConnID };

/// All errors that can happen when receiving messages over the wire
/// These will be broadcast to observers, so you can act upon them if necessary.
/// This type is a bit special as an error type, as here it is not being returned
/// in [`Result`] since the caller is remote. It is returned serialized
/// over the wire and broadcast to observers.
//
#[ derive( Debug, Clone, PartialEq, Eq, Serialize, Deserialize ) ]
//
pub enum ConnectionError
{
	/// An error deserializing the incoming actor message. This means the stream might be corrupt,
	/// so the connection will be closed.
	//
	Deserialize{ sid: Option<ServiceID>, cid: Option<ConnID> },

	/// An error deserializing the incoming data.
	//
	DeserializeWireFormat{ context: String },

	/// Your request could not be processed. This might mean spawning a future failed,
	/// downcasting a Receiver failed or other errors that are clearly not the fault
	/// of the remote peer.
	//
	InternalServerError{ sid: Option<ServiceID>, cid: Option<ConnID> },

	// The connection timed out while waiting for a response to a Call.
	// This will actually be used internally when an outgoing call times out, since we need to
	// send that over the channel which takes this error type. RemoteAddress will translate this in
	// a PeerErr. Client code should never observe this variant.
	//
	#[ doc( hidden ) ]
	//
	Timeout{ sid: ServiceID },

	/// We don't provide this service.
	//
	UnknownService{ sid: Option<ServiceID>, cid: Option<ConnID> },

	/// We don't provide this service.
	//
	PubSubNoCall{ sid: Option<ServiceID>, cid: Option<ConnID> },
}



impl std::error::Error for ConnectionError {}


impl fmt::Display for ConnectionError
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match &self
		{
			ConnectionError::Deserialize{ sid, cid } =>

				write!( f, "Remote failed to deserialize your actor message (sid: {:?}, cid: {:?}).", sid, cid ),

			ConnectionError::DeserializeWireFormat{ context } =>

				write!( f, "Remote failed to deserialize the wire format: {}", context ),

			ConnectionError::InternalServerError{ sid, cid } =>

				write!( f, "Remote ran into an internal server error (this isn't your fault). More information should be in their logs (sid: {:?}, cid: {:?}).", sid, cid ),

			ConnectionError::Timeout{ sid } =>

				write!( f, "Timed out waiting for a response to a call (sid: {}).", sid ),

			ConnectionError::UnknownService{ sid, .. } =>

				write!( f, "Remote does not expose the service you are trying to call (sid: {:?}).", sid ),

			ConnectionError::PubSubNoCall{ sid, .. } =>

				write!( f, "Remote broadcasts this message type using thespis_remote::PubSub which does not support the `call` operation. Only `send` is supported (sid: {:?}).", sid ),
		}
	}
}
