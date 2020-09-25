use crate::{ import::*, * };


/// Represents a timeout for an outgoing call. Tells the peer to remove the channel waiting
/// for a response from a remote from our hashmap to avoid memory leaks. Dropping it will
/// wake up the client code that is waiting for it.
//
#[ derive( Debug ) ]
//
pub(crate) struct Timeout
{
	pub (crate) cid: ConnID,
	pub (crate) sid: ServiceID,
}

impl Message for Timeout
{
	type Return = ();
}



/// Handler for timeout on outgoing calls.
//
impl<Wf: Send> Handler<Timeout> for Peer<Wf>
{
	fn handle( &mut self, msg: Timeout ) -> Return< '_, <Timeout as Message>::Return >
	{
		trace!( "{}: starting Handler<Timeout> for cid: {}", self.identify(), &msg.cid );

		async move
		{
			if let Some( tx ) = self.responses.remove( &msg.cid )
			{
				// If this fails, the receiver is already gone, so ignore the result.
				//
				let _ = tx.send( Err( ConnectionError::Timeout{ sid: msg.sid } ) );
			}

		}.boxed()
	}
}
