use crate::{ import::*, *, peer::RequestError };


/// A connection error from the remote peer
///
/// This includes failing to deserialize our messages, failing to relay, unknown service, ...
//
#[ derive( Debug ) ]
//
pub struct IncomingCall<Wf>
{
	pub(crate) frame : Wf                             ,
	pub(crate) sid   : ServiceID                      ,
	pub(crate) cid   : ConnID                         ,
	pub(crate) permit: Option< OwnedSemaphorePermit > ,
}


impl<Wf: WireFormat> Message for IncomingCall<Wf>
{
	///
	//
	type Return = ();
}


///
//
impl<Wf: WireFormat + Send + 'static> Handler<IncomingCall<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, mut msg: IncomingCall<Wf> ) -> <IncomingCall<Wf> as Message>::Return
	{
		if self.closed { return }

		trace!( "{}: Incoming Call, sid: {}, cid: {}", self.identify(), msg.sid, msg.cid );

		let ctx = self.ctx( msg.sid, msg.cid, "Peer: Handle incoming call" );


		// Find our handler.
		//
		let sm = match self.services.get( &msg.sid )
		{
			Some( sm ) => sm,

			// service_id unknown => send back and log error
			//
			None =>
			{
				let err = PeerErr::UnknownService{ ctx };

				return self.handle( RequestError::from( err ) ).await;
			}
		};


		if sm.apply_backpressure() {
		if let Some( p ) = msg.permit
		{
			self.permits.push(p);
		}}

		else { drop( msg.permit.take() ); }


		// Get future from service map.
		//
		let fut = match sm.call_service( msg.frame, ctx.clone() )
		{
			Ok (f) => f,
			Err(e) => return self.handle( RequestError::from(e) ).await,
		};


		// Call handling actor,
		//
		if self.nursery.nurse( fut ).is_err()
		{
			let err = PeerErr::Spawn{ ctx	};

			self.handle( RequestError::from( err ) ).await;
		}
	}
}
