use crate::{ import::*, * };


/// Type representing the outgoing call. Used by a recipient to a remote service to communicate
/// an outgoing call to [Peer]. Also used by [Peer] to call a remote service when relaying.
///
/// Normally you don't use this directly, but use the RemoteAddress to call
/// remote services.
//
#[ derive( Debug ) ]
//
pub struct Call
{
	mesg: BytesFormat,
}

impl Message for Call
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = Result< oneshot::Receiver<Result<BytesFormat, ConnectionError>>, PeerErr >;
}

impl Call
{
	/// Create a new Call to send an outgoing message over the peer.
	//
	pub fn new( mesg: BytesFormat ) -> Self
	{
		Self{ mesg }
	}
}



/// Handler for outgoing Calls
///
/// If the sending to the remote succeeds, you get back a oneshot receiver.
///
/// If sending to the remote fails, you get a PeerErr.
/// If the connection gets dropped before the answer comes, the oneshot::Receiver will err with Canceled.
/// If the remote fails to process the message, you will get a ConnectionError out of the channel.
//
impl Handler<Call> for Peer
{
	#[async_fn] fn handle( &mut self, call: Call ) -> <Call as Message>::Return
	{
		let identity = self.identify();

		trace!( "{}: polled Handler<Call>", &identity );

		// we no longer have our address, we're shutting down. we can't really do anything
		// without our address we won't have the sink for the connection either. We can
		// no longer send outgoing messages. Don't process any.
		//
		if self.closed
		{
			let ctx = self.ctx( None, None, "Handler<Call> for Peer" );

			return Err( PeerErr::ConnectionClosed{ ctx } );
		};


		// If self.closed is false, there should always be an address.
		//
		let mut self_addr = self.addr.as_ref().unwrap().clone();
		let     conn_id   = call.mesg.conn_id();
		let     sid       = call.mesg.service();

		// Otherwise the remote will consider it a send, and it's reserved anyway.
		//
		debug_assert!( conn_id != ConnID::null() );

		self.send_msg( call.mesg ).await?;

		// If the above succeeded, store the other end of the channel
		//
		let (sender, receiver) = oneshot::channel::< Result<BytesFormat, ConnectionError> >() ;


		// send a timeout message to ourselves.
		//
		let delay = self.timeout    ;
		let cid   = conn_id.clone() ;
		let sid2  = sid.clone()     ;

		let task = async move
		{
			Delay::new( delay ).await;

			if self_addr.send( super::Timeout{ cid, sid } ).await.is_err()
			{
				error!( "{}: Failed to send timeout to self.", &identity );
			}

			Ok(Response::Nothing)
		};


		self.nursery.nurse( task ).map_err( |_|
		{
			let ctx = self.ctx( sid2, None, "timeout for outgoing Call" );

			PeerErr::Spawn{ ctx }

		})?;


		self.responses.insert( conn_id, sender );

		Ok( receiver )
	}
}
