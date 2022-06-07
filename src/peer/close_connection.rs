use super::*;

/// Control message for [Peer]. The peer needs it's own address for normal operation,
/// so normally it will never drop, even if you drop all addresses you have of it.
/// Since it will never drop, it's mailbox will never stop listening for incoming messages
/// and the task will never complete, preventing your program from shutting down naturally.
///
/// With this message you can tell the peer to drop it's copy of it's own address. You still
/// have to drop your copies... otherwise the peer won't be dropped, but the peer will no
/// longer accept incoming Calls. Sends will still be processed, because once they have
/// arrived, the connection is no longer needed for them to be processed.
///
/// On an incoming call, an error shall be sent back to the other process.
///
/// The peer will also drop it's outgoing Sink, so the other end of the connection
/// will be notified that we close it.
///
/// If the remote closes the connection, all of this will happen automatically.
//
#[ derive( Debug ) ]
//
pub struct CloseConnection
{
	/// informs the peer whether the connection was closed remotely. If you close
	/// manually, set to false. The main effect of this is that the peer will send
	/// PeerEvents::ConnectionClosedByRemote to observers instead of PeerEvent::ConnectionClosed.
	//
	pub remote: bool,
	pub reason: String,
}

impl Message for CloseConnection { type Return = (); }



impl<Wf: WireFormat> Handler<CloseConnection> for Peer<Wf>
{
	#[async_fn] fn handle( &mut self, msg: CloseConnection )
	{
		trace!( "{}: CloseConnection, by remote: {}, reason: {}", self.identify(), msg.remote, &msg.reason );

		self.closed = true;

		// Since we don't close it, it shouldn't be closed.
		//
		if msg.remote { self.pharos.send( PeerEvent::ClosedByRemote ).await.expect( "pharos not closed" ) }
		else          { self.pharos.send( PeerEvent::Closed         ).await.expect( "pharos not closed" ) }


		// Try to close the connection properly
		//
		if let Some(out) = &mut self.outgoing
		{
			match out.close().await
			{
				Ok(_) => {}

				Err(source) =>
				{
					let ctx = self.ctx( None, None, "Closing the connection" );
					let err = PeerErr::WireFormat{ ctx, source };

					// We didn't close it, so the expect should be fine.
					//
					self.pharos.send( PeerEvent::Error(err) ).await.expect( "pharos not closed" );
				}
			};

			self.outgoing = None;
		};


		self.nursery.close_nursery();


		// try to drop close our mailbox and drop ourselves
		//
		self.addr = None;

		if let Some( duration ) = self.grace_period {
		if let Some( stream   ) = self.nursery_stream.take()
		{
			let delay = Delay::new( duration );

			pin_mut!( delay  );
			pin_mut!( stream );

			futures::future::select( delay, stream ).await;
		}}

		self.nursery_stream = None;


		// Also clear everything else, because services might have our address, because they
		// want to send stuff over the network, so if we keep them alive, they will keep us
		// alive. This breaks that cycle.
		//
		self.services .clear();
		self.responses.clear();
	}
}
