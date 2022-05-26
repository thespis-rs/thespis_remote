use crate::{ CallResponse, CborWF };

/// A type to unify the two types of responses that can be returned by spawned tasks that
/// process a request.
//
#[ derive(Debug) ]
//
pub enum Response<Wf = CborWF>
{
	/// Eg. a relay, we are not using back pressure for this in the peer.
	//
	WireFormat(Wf),

	/// Response to a call from a local actor. This uses back pressure in the peer.
	//
	CallResponse(CallResponse<Wf>),

	/// Nothing, eg. task handles a send.
	//
	Nothing,
}
