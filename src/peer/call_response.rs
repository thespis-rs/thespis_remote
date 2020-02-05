use crate::{ import::*, * };


/// Type representing the response to a remote request. It is simply a wrapped Wireformat, but we
/// need to mark it with a type to keep track of the backpressure.
//
#[ derive( Debug ) ]
//
pub struct CallResponse
{
	msg: WireFormat,
}


impl Message for CallResponse
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = Result<(), ThesRemoteErr>;
}


impl CallResponse
{
	/// Create a new CallResponse to send an outgoing message over the peer.
	//
	pub fn new( msg: WireFormat ) -> Self
	{
		Self{ msg }
	}
}



/// Handler for outgoing CallResponse. Compared to simply sending out, this will notify the
/// backpressure that we are done to free a slot for new requests.
//
impl Handler<CallResponse> for Peer
{
	fn handle( &mut self, wrap: CallResponse ) -> Return< '_, <CallResponse as Message>::Return >
	{
		async move
		{
			trace!( "{}: sending OUT CallResponse", self.identify() );

			let res = self.send_msg( wrap.msg ).await;

			if let Some( ref bp ) = self.backpressure
			{
				trace!( "Liberate slot for backpressure." );

				bp.add_slots( NonZeroUsize::new( 1 ).expect( "1 != 0" ) );
			}

			res

		}.boxed()
	}
}
