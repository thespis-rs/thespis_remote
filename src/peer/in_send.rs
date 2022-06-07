use crate::{ import::*, *, peer::RequestError };


/// A connection error from the remote peer
///
/// This includes failing to deserialize our messages, failing to relay, unknown service, ...
//
#[ derive( Debug ) ]
//
pub struct IncomingSend<Wf>
{
	pub(crate) frame: Wf,
	pub(crate) sid  : ServiceID,
}


impl<Wf: WireFormat> Message for IncomingSend<Wf>
{
	///
	//
	type Return = ();
}


///
//
impl<Wf: WireFormat + Send + 'static> Handler<IncomingSend<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, msg: IncomingSend<Wf> ) -> <IncomingSend<Wf> as Message>::Return
	{
		let identity = self.identify();

		trace!( "{}: Incoming Send, sid: {}", &identity, &msg.sid );

		let ctx = self.ctx( msg.sid, None, "Peer: Handle incoming send" );

		let sm = match self.services.get( &msg.sid )
		{
			Some( sm ) => sm,

			// service_id unknown => send back and log error
			//
			None =>
			{
				let err = PeerErr::UnknownService{ ctx };

				self.handle( RequestError::from( err ) ).await;

				return;
			}
		};


		// Send to handling actor,
		//
		let fut = match sm.send_service( msg.frame, ctx )
		{
			Ok(f) => f,

			Err(e) =>
			{
				let ctx = self.ctx( msg.sid, None, "sm.send_service" );

				let err = match e
				{
					PeerErr::NoHandler  {..} => PeerErr::NoHandler  {ctx} ,
					PeerErr::Deserialize{..} => PeerErr::Deserialize{ctx} ,
					_                        => unreachable!()            ,
				};


				// If we are no longer around, just log the error.
				//
				return self.handle( RequestError::from(err) ).await;
			}
		};


		if self.nursery.nurse( fut ).is_err()
		{
			let ctx = self.ctx( msg.sid, None, "sm.send_service" );

			let err = PeerErr::Spawn { ctx };

			// If we are no longer around, just log the error.
			//
			self.handle( RequestError::from(err) ).await
		}
	}
}
