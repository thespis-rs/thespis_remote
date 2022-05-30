use crate::{ import::*, * };


/// Type representing the outgoing call. Used by a recipient to a remote service to communicate
/// an outgoing call to [Peer]. Also used by [Peer] to call a remote service when relaying.
///
/// Normally you don't use this directly, but use the RemoteAddress to call
/// remote services.
//
#[ derive( Debug ) ]
//
pub struct Call<Wf>
{
	 wf   : Wf              ,
	_ghost: PhantomData<Wf> ,
}

impl<Wf: WireFormat> Message for Call<Wf>
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = Result< oneshot::Receiver<Result<Wf, ConnectionError>>, PeerErr >;
}

impl<Wf: WireFormat> Call<Wf>
{
	/// Create a new Call to send an outgoing message over the peer.
	//
	pub fn new( wf: Wf ) -> Self
	{
		Self{ wf, _ghost: PhantomData }
	}

	/// Get the service id.
	//
	pub fn service( &self ) -> ServiceID
	{
		self.wf.sid()
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
impl<Wf: WireFormat + Send + 'static> Handler<Call<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, mut call: Call<Wf> ) -> <Call<Wf> as Message>::Return
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


		let mut cid = ConnID::from( self.conn_id_counter.fetch_add(1, Relaxed) );
		let     sid = call.wf.sid();

		// We wrapped round.
		// It must not be 0 otherwise the remote will consider it a send, and it's reserved.
		//
		if cid.is_null()
		{
			cid = ConnID::from( self.conn_id_counter.fetch_add( 1, Relaxed ) );
		}

		call.wf.set_cid( cid );

		self.send_msg( call.wf ).await?;

		// If the above succeeded, store the other end of the channel
		//
		let (sender, receiver) = oneshot::channel::< Result<Wf, ConnectionError> >() ;


		// send a timeout message to ourselves.
		//
		let delay = self.timeout;

		// If self.closed is false, there should always be an address.
		//
		let mut self_addr = self.addr.as_ref().unwrap().clone();

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
			let ctx = self.ctx( sid, None, "timeout for outgoing Call" );

			PeerErr::Spawn{ ctx }

		})?;


		self.responses.insert( cid, sender );

		Ok( receiver )
	}
}
