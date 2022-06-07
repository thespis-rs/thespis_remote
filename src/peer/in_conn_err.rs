use crate::{ import::*, * };


/// A connection error from the remote peer
///
/// This includes failing to deserialize our messages, failing to relay, unknown service, ...
//
#[ derive( Debug ) ]
//
pub struct IncomingConnErr<Wf>
{
	pub(crate) frame: Wf,
	pub(crate) cid  : ConnID,
}


impl<Wf: WireFormat> Message for IncomingConnErr<Wf>
{
	///
	//
	type Return = ();
}


///
//
impl<Wf: WireFormat + Send + 'static> Handler<IncomingConnErr<Wf>> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, msg: IncomingConnErr<Wf> ) -> <IncomingConnErr<Wf> as Message>::Return
	{
		let serialized = msg.frame.msg();

		// We can correctly interprete the error
		//
		if let Ok( err ) = serde_cbor::from_slice::<ConnectionError>( serialized )
		{
			// We need to report the connection error to the caller
			//
			if let Some( channel ) = self.responses.remove( &msg.cid )
			{
				// If this returns an error, it means the receiver was dropped, so if they no longer
				// care for the result, neither do we, so ignoring the result.
				//
				let _ = channel.send( Err( err ) );

				// Since this was not our error, just relay the response.
				//
				return
			}

			// Notify observers
			//
			let shine = PeerEvent::RemoteError( err );

			// If pharos is closed, we already panicked... so except is fine.
			//
			self.pharos.send( shine ).await.expect( "pharos not closed" );

		}

		// Since we can't deserialize it, we can't do much except log.
		//
		else
		{
			let ctx   = self.ctx( None, msg.cid, "We received an error message from a remote peer, but couldn't deserialize it" );
			let err   = PeerErr::Deserialize{ ctx };
			let shine = PeerEvent::Error(err);

			// If pharos is closed, we already panicked... so except is fine.
			//
			self.pharos.send( shine ).await.expect( "pharos not closed" );
		}
	}
}
