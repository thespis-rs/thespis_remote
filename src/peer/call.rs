use crate::{ import::*, * };


/// Type representing the outgoing call. Used by a recipient to a remote service to communicate
/// an outgoing call to [Peer]. Also used by [Peer] to call a remote service when relaying.
///
/// MS must be of the same type as the type parameter on [Peer].
///
/// Normally you don't use this directly, but use the recipient a service map gives you to call
/// remote services.
//
#[ derive( Debug ) ]
//
pub struct Call
{
	mesg: WireFormat,
}

impl Message for Call
{
	type Return = Result< oneshot::Receiver<Result<WireFormat, ConnectionError>>, ThesRemoteErr >;
}

impl Call
{
	/// Create a new Call to send an outgoing message over the peer.
	//
	pub fn new( mesg: WireFormat ) -> Self
	{
		Self{ mesg }
	}
}



/// Handler for outgoing Calls
///
/// If the sending to the remote succeeds, you get back a oneshot receiver.
///
/// If sending to the remote fails, you get a ThesRemoteErr.
/// If the connection gets dropped before the answer comes, the onshot::Receiver will err with Cancelled.
/// If the remote fails to process the message, you will get a ConnectionError out of the channel.
//
impl Handler<Call> for Peer
{
	fn handle( &mut self, call: Call ) -> Return< '_, <Call as Message>::Return >
	{
		trace!( "peer: starting Handler<Call>" );

		Box::pin( async move
		{
			trace!( "peer: polled Handler<Call>" );

			// Fallible operations first
			// Can fail to deserialize connection id from the outgoing call.
			//
			let conn_id = call.mesg.conn_id()?;
			self.send_msg( call.mesg ).await?;

			// If the above succeeded, store the other end of the channel
			//
			let (sender, receiver) = oneshot::channel::< Result<WireFormat, ConnectionError> >() ;

			// TODO: Probably need to keep some timeout mechanism for when responses never come...
			//
			self.responses.insert( conn_id, sender );

			Ok( receiver )

		})
	}
}
