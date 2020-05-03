use crate::{ import::*, * };

/// Errors that happen on  processing requests. This type is not in the public API.
/// Since both Peer and PeerErr are public and we don't want users to be able
/// to send this type to peer, we wrap it.
///
/// When requests come in over the network, we spawn a new task to handle each request.
/// These tasks must be able to communicate errors to the rest of the program, but
/// since they originate from a network request, we can't bubble up errors.
///
/// So Peer implements Handler for these and they will send errors as actor messages to
/// be processed here.
///
/// In the public API there is PeerErr for the local process and ConnectionError which is
/// a serializable error to be sent to remotes. ConnectionError also strips some information that
/// should not be leaked to a remote by grouping a number of errors into InternalServerError.
///
/// Overview request errors:
///
/// - Incoming requests can be malformed in a number of ways:
///   - missing or unknown sid/cid
///   - deserialization errors:
///     - length not u64
///     -
///
/// - spawn errors can happen when spawning tasks to handle the request
///
/// - handlers might panic
///
/// - remote connection might shut down before we can send a response.
///
///
/// - when relaying all of the above can happen in our own process, as well as in the
///   provider, plus io errors on the connection, or the connection to the provider
///   might be gone all together.
///
/// For all of these we need to act appropriately.
/// - all errors are logged as is.
/// - all errors should be reported on pharos as events.
/// - some errors should be reported back to the remote.
/// - some errors should cause the connection to be closed.
//
#[ derive( Debug, Clone ) ]
//
pub struct RequestError
{
	pub(crate) error: PeerErr
}

impl Message for RequestError
{
	type Return = ();
}


/// Handler for errors that happened during request processing.
//
impl Handler<RequestError> for Peer
{
	fn handle( &mut self, msg: RequestError ) -> Return<'_, ()> { async move
	{
		// All errors get reported through pharos.
		// expect: pharos shouldn't be closed unless we close it and we don't.
		//
		self.pharos.send( PeerEvent::Error( msg.error.clone() ) ).await.expect( "pharos not closed" );

		// If it was a send, don't send errors to the remote. Only call buys into feedback.
		//
		let cid = match &msg.error.ctx().cid
		{
			Some(c) => c.clone(),
			None    => return,
		};


		// Send errors back to the remote.
		//
		match msg.error
		{
			// TODO: need to decide what to do.
			//
			PeerErr::WireFormat{..} =>
			{
				// Report to remote and close connection as the stream is no longer coherent.
				//
				let err = ConnectionError::DeserializeWireFormat{ context: msg.error.remote_err() };

				// If the error happened in the codec, there won't be a cid, but if it happens
				// while deserializing the actor message, we will already have a cid.
				//
				self.send_err( cid, &err, true ).await;
			}


			PeerErr::Deserialize{ ctx } =>
			{
				// Report to remote and close connection as the stream is no longer coherent.
				//
				let err = ConnectionError::Deserialize{ sid: ctx.sid, cid: cid.clone().into() };

				// If the error happened in the codec, there won't be a cid, but if it happens
				// while deserializing the actor message, we will already have a cid.
				//
				self.send_err( cid, &err, false ).await;
			}


			PeerErr::Spawn { ctx } =>
			{
				// Report to remote and close connection. When we can't spawn, we can't process
				// any more incoming message, so it seems sensible to close the connection.
				//
				let err = ConnectionError::InternalServerError{ sid: ctx.sid, cid: cid.clone().into() };

				self.send_err( cid, &err, true ).await;
			}


			  PeerErr::RelayGone  { ctx, .. }
			| PeerErr::NoHandler  { ctx     }
			| PeerErr::HandlerDead{ ctx     } =>
			{
				// Report to remote, we don't close the connection because we might expose other
				// services that are still operational, or the actor might be in the process of
				// being restarted.
				//
				let err = ConnectionError::InternalServerError{ sid: ctx.sid, cid: cid.clone().into() };

				self.send_err( cid, &err, false ).await;
			}


			// This means we fail to serialize the response of a call. This is no error from the
			// remote, but from the local process.
			//
			PeerErr::Serialize{ ctx } =>
			{
				// Report to remote, we don't close the connection because this might work again later.
				//
				let err = ConnectionError::InternalServerError{ sid: ctx.sid, cid: cid.clone().into() };

				self.send_err( cid, &err, false ).await;
			}


			PeerErr::UnknownService{ ctx } =>
			{
				// This is not fatal, NOT closing the connection.
				//
				let err = ConnectionError::UnknownService{ sid: ctx.sid, cid: cid.clone().into() };

				self.send_err( cid, &err, false ).await;
			}

			// We shouldn't accept any other errors unknowingly.
			// Especially we log the error above the match, so if there is other
			// error types, we really need to add the variant to the match.
			//
			_ => { unreachable!() }
		}

	}.boxed() }
}



impl From<PeerErr> for RequestError
{
	fn from( error: PeerErr ) -> Self
	{
		Self { error }
	}
}
