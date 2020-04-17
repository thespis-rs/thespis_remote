use crate::{ import::*, * };

/// Errors that happen on  processing requests. This type is not in the public API.
/// Since both Peer and ThesRemoteErr are public and we don't want users to be able
/// to send this type to peer, we wrap it.
///
/// When requests come in over the network, we spawn a new task to handle each request.
/// These tasks must be able to communicate errors to the rest of the program, but
/// since they originate from a network request, we can't bubble up errors.
///
/// So Peer implements Handler for these and they will send errors as actor messages to
/// be processed here.
///
/// In the public API there is ThesRemoteErr for the local process and ConnectionError which is
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
	pub(crate) error: ThesRemoteErr
}

impl Message for RequestError
{
	type Return = ();
}


/// Handler for incoming messages
//
impl Handler<RequestError> for Peer
{
	fn handle( &mut self, msg: RequestError ) -> Return<'_, ()> { async move
	{
		// All errors get logged.
		//
		error!( "{}: {}", self.identify(), msg.error );

		// All errors get reported through pharos.
		// expect: pharos shouldn't be closed unless we close it and we don't.
		//
		self.pharos.send( PeerEvent::Error( msg.error.clone() ) ).await.expect( "pharos not closed" );


		match msg.error
		{
			ThesRemoteErr::MessageSizeExceeded{..} =>
			{
				// Report to remote and close connection as the stream is no longer coherent.
				//
				let err = ConnectionError::DeserializeWireFormat{ context: msg.error.remote_err() };

				self.send_err( ConnID::null(), &err, true ).await;
			}


			ThesRemoteErr::DeserializeWireFormat{ ref ctx } =>
			{
				let cid = ctx.cid.as_ref().cloned().unwrap_or_else( ConnID::null );

				// Report to remote and close connection as the stream is no longer coherent.
				//
				let err = ConnectionError::DeserializeWireFormat{ context: msg.error.remote_err() };

				// If the error happened in the codec, there won't be a cid, but if it happens
				// while deserializing the actor message, we will already have a cid.
				//
				self.send_err( cid, &err, true ).await;
			}


			ThesRemoteErr::Deserialize{ ctx } =>
			{
				// Report to remote and close connection as the stream is no longer coherent.
				//
				let err = ConnectionError::Deserialize{ sid: ctx.sid, cid: ctx.cid.clone() };

				// If the error happened in the codec, there won't be a cid, but if it happens
				// while deserializing the actor message, we will already have a cid.
				//
				self.send_err( ctx.cid.unwrap_or_else( ConnID::null ), &err, false ).await;
			}


			  ThesRemoteErr::Downcast{ ctx }
			| ThesRemoteErr::Spawn   { ctx } =>
			{
				// Report to remote and close connection. When we can't spawn, we can't process
				// any more incoming message, so it seems sensible to close the connection.
				//
				let err = ConnectionError::InternalServerError{ sid: ctx.sid, cid: ctx.cid.clone() };

				self.send_err( ctx.cid.unwrap_or_else( ConnID::null ), &err, true ).await;
			}


			  ThesRemoteErr::RelayGone      { ctx, .. }
			| ThesRemoteErr::NoHandler      { ctx     }
			| ThesRemoteErr::ServiceMapDead { ctx     }
			| ThesRemoteErr::HandlerDead    { ctx     } =>
			{
				// Report to remote, we don't close the connection because we might expose other
				// services that are still operational, or the actor might be in the process of
				// being restarted.
				//
				let err = ConnectionError::InternalServerError{ sid: ctx.sid, cid: ctx.cid.clone() };

				self.send_err( ctx.cid.unwrap_or_else( ConnID::null ), &err, false ).await;
			}


			// This means we fail to serialize the response of a call. This is no error from the
			// remote, but from the local process.
			//
			ThesRemoteErr::Serialize{ ctx } =>
			{
				// Report to remote, we don't close the connection because this might work again later.
				//
				let err = ConnectionError::InternalServerError{ sid: ctx.sid, cid: ctx.cid.clone() };

				self.send_err( ctx.cid.unwrap_or_else( ConnID::null ), &err, false ).await;
			}


			ThesRemoteErr::UnknownService{ ctx } =>
			{
				// This is not fatal, NOT closing the connection.
				//
				let err = ConnectionError::UnknownService{ sid: ctx.sid, cid: ctx.cid.clone() };

				self.send_err( ctx.cid.unwrap_or_else( ConnID::null ), &err, false ).await;
			}

			// We shouldn't accept any other errors unknowingly.
			// Especially we log the error above the match, so if there is other
			// error types, we really need to add the variant to the match.
			//
			_ => { unreachable!() }
		}

	}.boxed() }
}



impl From<ThesRemoteErr> for RequestError
{
	fn from( error: ThesRemoteErr ) -> Self
	{
		Self { error }
	}
}
