use crate::{ import::*, * };


///
//
#[ derive( Debug ) ]
//
pub struct IncomingCallResponse<Wf>
{
	pub(crate) frame: Wf,
	pub(crate) cid  : ConnID,
}


impl<Wf: WireFormat> Message for IncomingCallResponse<Wf>
{
	///
	//
	type Return = ();
}


///
//
impl<Wf: WireFormat + Send + 'static> Handler<IncomingCallResponse<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, msg: IncomingCallResponse<Wf> ) -> <IncomingCallResponse<Wf> as Message>::Return
	{
		// it's a succesful response to a (relayed) call
		//
		if let Some( channel ) = self.responses.remove( &msg.cid )
		{
			// It's a response
			//
			trace!( "{}: Incoming Return", self.identify() );

			// Normally if this fails it means the receiver of the channel was dropped...
			//
			if channel.send( Ok(msg.frame) ).is_err()
			{
				warn!( "{}: Received response for dead actor, cid: {}.", self.identify(), msg.cid );
			}
		}

		// There is a CID, so it's a response, but it's not in our self.responses, so it has timed out.
		// We are no longer waiting for this response, so we can only drop it.
		//
		else
		{
			warn!( "{}: Received response for a timed out outgoing request, cid: {}. Dropping response.", self.identify(), msg.cid );
		}
	}
}
