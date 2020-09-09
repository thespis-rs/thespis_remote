use crate::{ import::*, * };


/// Type representing the response to a remote request. It is simply a wrapped BytesFormat, but we
/// need to mark it with a type to keep track of the backpressure.
//
#[ derive( Debug ) ]
//
pub struct CallResponse<Wf>
{
	msg: Wf,
}


impl<Wf: WireFormat> Message for CallResponse<Wf>
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = Result<(), PeerErr>;
}


impl<Wf> CallResponse<Wf>
{
	/// Create a new CallResponse to send an outgoing message over the peer.
	//
	pub fn new( msg: Wf ) -> Self
	{
		Self{ msg }
	}
}



/// Handler for outgoing CallResponse. Compared to simply sending out, this will notify the
/// backpressure that we are done to free a slot for new requests.
//
impl<Wf: WireFormat + Send + 'static> Handler<CallResponse<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, wrap: CallResponse<Wf> ) -> <CallResponse<Wf> as Message>::Return
	{
		trace!( "{}: sending OUT CallResponse", self.identify() );

		let res = self.send_msg( wrap.msg ).await;

		if let Some( ref bp ) = self.backpressure
		{
			trace!( "Liberate slot for backpressure." );

			bp.add_slots( NonZeroUsize::new( 1 ).expect( "1 > 0" ) );
		}

		res
	}
}
