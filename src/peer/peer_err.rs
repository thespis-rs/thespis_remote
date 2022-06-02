use crate::{ import::*, ConnID, ServiceID, ConnectionError, WireErr };


/// Errors that can happen in thespis_impl.
//
#[ derive( Debug, Clone, PartialEq, Eq ) ]
//
#[ non_exhaustive ]
//
pub enum PeerErr
{
	/// Cannot use peer after the connection is closed.
	//
	ConnectionClosed
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// Failed to deserialize an Actor message. The message data will be dropped and the remote will be notified of the error.
	/// The connection shall remain functional.
	//
	Deserialize
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// Cannot deliver because the handling actor is no longer running.
	//
	HandlerDead
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// No handler has been set for this service.
	/// If you use the provided ServiceMap implementations, you should only see this if you
	/// use a closure with RelayMap and it returns `None`, because otherwise they don't
	/// advertise services for which they haven't got a handler.
	//
	NoHandler
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// When trying to send a message to the peer, it errored. This means either the peer has panicked or you dropped it's inbox.
	//
	PeerGone
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// Failed to relay a request because the connection to the relay has been closed.
	//
	RelayGone
	{
		ctx       : PeerErrCtx ,
		relay_id  : usize      ,
		relay_name: Arc<str>   ,
	},

	/// An error happened when a remote tried to process your message.
	//
	Remote
	{
		ctx: PeerErrCtx      ,
		err: ConnectionError ,
	},

	/// Failed to serialize actor message.
	//
	Serialize
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// Failed to spawn a task.
	//
	Spawn
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},


	/// TODO: clean up docs and error message.
	/// This allows also returning all ThesErr kinds when returning a PeerErr. eg. Often
	/// operations from remote will use call and send which give mailbox errors, so it's good to
	/// be able to return those as well as the more remote specific errors that might happen in
	/// the same method.
	//
	ThesErr
	{
		ctx   : PeerErrCtx   ,
		source: Arc<ThesErr> , // it does not implement clone.
	},

	/// An operation timed out. Currently used for outgoing calls.
	//
	Timeout
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// Cannot deliver message to unknown service.
	//
	UnknownService
	{
		/// The contex in which the error happened.
		//
		ctx: PeerErrCtx
	},

	/// Error for encoding/decoding the bytestream or underlying IO errors.
	//
	WireFormat
	{
		/// The contex in which the error happened.
		//
		ctx   : PeerErrCtx ,
		source: WireErr    ,
	},

	/// A PubSub allows you to have a publish subscribe model. This implies
	/// that one sender can have multiple subscribers, which means we cannot
	/// implement a request-response type communication. Which subscriber
	/// is formulating the response?
	///
	/// Thus you can only use `Sink::send` but not `Address::call` over
	/// a PubSub.
	//
	PubSubNoCall
	{
		/// The contex in which the error happened.
		//
		ctx   : PeerErrCtx ,
	},

	/// The semaphore for the backpressure has been closed externally.
	//
	BackpressureClosed
	{
		/// The contex in which the error happened.
		//
		ctx   : PeerErrCtx ,
	}
}



impl std::error::Error for PeerErr {}


impl fmt::Display for PeerErr
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match &self
		{
			PeerErr::ConnectionClosed{ ctx } =>

				write!( f, "Cannot use peer after the connection is closed, operation.{}", ctx ),

			PeerErr::Deserialize{ ctx } =>

				write!( f, "Failed to deserialize an Actor message.{}", ctx ),

			PeerErr::HandlerDead{ ctx } =>

				write!( f, "Cannot deliver because the handling actor is no longer running.{}", ctx ),

			PeerErr::NoHandler{ ctx } =>

				write!( f, "No handler has been set for this service.{}", ctx ),

			PeerErr::PeerGone{ ctx } =>

				write!( f, "The Peer actor has panicked.{}", ctx ),

			PeerErr::RelayGone{ ctx, relay_id, relay_name } =>

				write!( f, "Failed to relay a request because the connection to the relay has been closed. context.{} relay_id: {}, relay_name: {:?}", ctx, relay_id, relay_name ),

			PeerErr::Remote{ err, ctx } =>

				write!( f, "A remote could not process a message we sent it{:?}.{}", err, ctx ),

			PeerErr::Serialize{ ctx } =>

				write!( f, "Failed to serialize:{}", ctx ),

			PeerErr::Spawn{ ctx } =>

				write!( f, "Failed to spawn task:{}", ctx ),

			PeerErr::ThesErr{ ctx, source } =>

				write!( f, "ThesErr {}.{}", source, ctx ),

			PeerErr::Timeout{ ctx } =>

				write!( f, "Operation Timed out.{}", ctx ),

			PeerErr::UnknownService{ ctx } =>

				write!( f, "Cannot deliver message to unknown service.{}", ctx ),

			PeerErr::WireFormat{ ctx, source } =>

				write!( f, "An error happened on the underlying stream: {}.{}", source, ctx ),

			PeerErr::PubSubNoCall{ ctx } =>

				write!( f, "PubSub does not support `Address::call` operation, only `Sink::send`.{}", ctx ),

			PeerErr::BackpressureClosed{ ctx } =>

				write!( f, "The semaphore for backpressure was closed externally.{}", ctx ),
		}
	}
}


impl PeerErr
{
	/// TODO: make sure we don't leak any info on other error variants, for WireErr, is there anything to hide?
	/// Produce a display string suitable for sending errors to a connected client. This omit's
	/// process specific information like actor id's.
	//
	pub fn remote_err( self ) -> String
	{
		match self
		{
			Self::WireFormat{ source, mut ctx } =>
			{
				ctx.peer_id   = None;
				ctx.peer_name = None;

				match source
				{
					WireErr::MessageSizeExceeded{ context, size, max_size } =>

						format!( "Maximum message size exceeded: context: {}, actual: {} bytes, allowed: {} bytes." , &context, &size, &max_size ),

					WireErr::Deserialize{..} =>

						format!( "Could not deserialize your message.{}", &ctx ),

					WireErr::Io{..} =>

						format!( "An error happened on the underlying transport.{}", &ctx ),
				}

			}

			Self::Deserialize{ ctx } =>
			{
				format!( "Could not deserialize your message.{}", &ctx )
			}

			_ => { unreachable!() }
		}
	}


	pub fn ctx( &self ) -> &PeerErrCtx
	{
		match self
		{
			PeerErr::ConnectionClosed   { ctx, .. } => ctx,
			PeerErr::Deserialize        { ctx, .. } => ctx,
			PeerErr::HandlerDead        { ctx, .. } => ctx,
			PeerErr::NoHandler          { ctx, .. } => ctx,
			PeerErr::PeerGone           { ctx, .. } => ctx,
			PeerErr::RelayGone          { ctx, .. } => ctx,
			PeerErr::Remote             { ctx, .. } => ctx,
			PeerErr::Serialize          { ctx, .. } => ctx,
			PeerErr::Spawn              { ctx, .. } => ctx,
			PeerErr::ThesErr            { ctx, .. } => ctx,
			PeerErr::Timeout            { ctx, .. } => ctx,
			PeerErr::UnknownService     { ctx, .. } => ctx,
			PeerErr::WireFormat         { ctx, .. } => ctx,
			PeerErr::PubSubNoCall       { ctx, .. } => ctx,
			PeerErr::BackpressureClosed { ctx, .. } => ctx,
		}
	}
}



#[ derive( Default, Debug, Clone, PartialEq, Eq ) ]
//
pub struct PeerErrCtx
{
	pub context  : Option< String    > ,
	pub peer_id  : Option< usize     > ,
	pub peer_name: Option< Arc<str>  > ,
	pub sid      : Option< ServiceID > ,
	pub cid      : Option< ConnID    > ,
}


impl PeerErrCtx
{
	pub fn context( mut self, context: impl Into<Option< String >> ) -> Self
	{
		self.context = context.into();
		self
	}

	pub fn peer_id( mut self, peer_id: impl Into<Option< usize >> ) -> Self
	{
		self.peer_id = peer_id.into();
		self
	}

	pub fn peer_name( mut self, peer_name: impl Into<Option< Arc<str> >> ) -> Self
	{
		self.peer_name = peer_name.into();
		self
	}

	pub fn sid( mut self, sid: impl Into<Option< ServiceID >> ) -> Self
	{
		self.sid = sid.into();
		self
	}

	pub fn cid( mut self, cid: impl Into<Option< ConnID >> ) -> Self
	{
		self.cid = cid.into();
		self
	}
}



impl fmt::Display for PeerErrCtx
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		if let Some( x ) = &self.context
		{
			write!( f, " Context: {}.",	x )?;
		}

		if let Some( x ) = self.peer_id
		{
			write!( f, " peer_id: {}.",	x )?;
		}

		if let Some( x ) = &self.peer_name
		{
			write!( f, " peer_name: {}.",	x )?;
		}

		if let Some( x ) = &self.sid
		{
			write!( f, " sid: {}.",	x )?;
		}

		if let Some( x ) = &self.cid
		{
			write!( f, " cid: {}.",	x )?;
		}

		Ok(())
	}
}

