//! This is a separate async task that will loop over the incoming stream and sends the
//! incoming messages to the mailbox of the peer.
//!
//!
//!
//!
//!
//
use super::
{
	in_call          ::IncomingCall         ,
	in_call_response ::IncomingCallResponse ,
	in_conn_err      ::IncomingConnErr      ,
	in_send          ::IncomingSend         ,
	*                                       ,
};


impl<Wf: WireFormat> Peer<Wf>
{


	/// The task that will listen to incoming messages on the network connection and send them to our
	/// the peer's address.
	//
	pub(crate) async fn listen_incoming
	(
		mut incoming: impl BoundsIn<Wf>         ,
		mut addr    : Addr<Peer<Wf>>            ,
		    bp      : Option< Arc<Semaphore> >  ,
	)
		-> Result<Response<Wf>, PeerErr>

	{
		// Stream over Result<Wf, WireErr>
		// From the codec.
		//
		while let Some(msg) = incoming.next().await
		{
			trace!( "{}: incoming message.", &addr );

			// Handle errors first.
			//
			let frame = match msg
			{
				Ok ( mesg  ) => mesg,
				Err( error ) =>
				{
					// Can be:
					// - WireErr::MessageSizeExceeded (Codec)
					// - WireErr::Deserialize (BytesFormat)
					// - WireErr::IO...
					//
					let ctx = Self::err_ctx( &addr.weak(), None, None, "Deserialize Incoming message or IO error.".to_string() );
					let err = PeerErr::WireFormat{ source: error, ctx };

					Self::send_to_self( &mut addr, RequestError::from( err ) ).await?;

					return Ok(Response::Nothing)
				}
			};



			let sid  = frame.sid();
			let cid  = frame.cid();
			let kind = frame.kind();

			// TODO: when we have benchmarks, verify if it's better to return boxed submethods here
			// rather than awaiting. Implies the rest of this method can run sync.
			//
			match kind
			{
				WireType::ConnectionError =>
				{
					Self::send_to_self( &mut addr, IncomingConnErr{ frame, cid } ).await?;
				}

				WireType::IncomingSend =>
				{
					Self::send_to_self( &mut addr, IncomingSend{ frame, sid } ).await?;
				}

				WireType::IncomingCall =>
				{
					let permit = match &bp
					{
						None => None,

						Some(b) =>
						{
							trace!( "check for backpressure" );

							let p = match Arc::clone(b).acquire_owned().await
							{
								Ok(p) => p,

								Err(_e) =>
								{
									error!( "{}: The semaphore for backpressure was closed externally.", Peer::identify_addr( &addr ) );

									let ctx = Self::err_ctx( &addr.weak(), None, None, "Peer::listen_incoming: backpressure semaphore is closed.".to_string() );

									let err = PeerErr::BackpressureClosed{ ctx };
									return Err(err);
								}
							};

							trace!( "backpressure allows progress now." );

							Some(p)
						}
					};

					Self::send_to_self( &mut addr, IncomingCall{ frame, cid, sid, permit } ).await?;
				}


				WireType::CallResponse =>
				{
					Self::send_to_self( &mut addr, IncomingCallResponse{ frame, cid } ).await?;
				}
			}
		}

		trace!( "{}:  incoming stream end, closing out.", &addr );

		// The connection was closed by remote, tell peer to clean up.
		//
		let res = addr.send( CloseConnection{ remote: true, reason: "Connection closed by remote.".to_string() } ).await;

		// As we hold an address, the only way the mailbox can already be shut
		// is if the peer panics, or the mailbox get's dropped. Since the mailbox
		// owns the Peer, if it's dropped, this task doesn't exist anymore.
		// Should never happen.
		//
		debug_assert!( res.is_ok() );

		Ok(Response::Nothing)
	}


	async fn send_to_self<T>
	(
		addr: &mut Addr<Peer<Wf>>,
		msg: T,
	)
	-> Result<(), PeerErr>

		where T: Message, Self: Handler<T>,

	{
		if let Err(e) = addr.send( msg ).await
		{
			error!( "{} Peer::listen_incoming: peer mailbox no longer taking messages.", Peer::identify_addr( &addr ) );

			let ctx = Self::err_ctx( &addr.weak(), None, None, "Peer::listen_incoming: peer mailbox no longer taking messages.".to_string() );

			let err = PeerErr::ThesErr{ ctx, source: Arc::new(e) };
			return Err(err);
		}

		Ok(())
	}
}
