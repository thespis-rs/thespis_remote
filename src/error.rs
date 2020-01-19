use crate::{ import::*, ConnID, ServiceID, ConnectionError };


/// Errors that can happen in thespis_impl.
//
#[ derive( Debug, Error, Clone, PartialEq, Eq ) ]
//
#[ non_exhaustive ]
//
pub enum ThesRemoteErr
{
	/// Cannot use peer after the connection is closed.
	//
	#[ error( "Cannot use peer after the connection is closed, operation.{ctx}" ) ]
	//
	ConnectionClosed
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// An error happened when a remote tried to process your message.
	//
	#[ error( "A remote could not process a message we sent it{err:?}{ctx}" ) ]
	//
	Remote
	{
		ctx: ErrorContext    ,
		err: ConnectionError ,
	},

	/// Failed to downcast. This indicates an error in thespis_remote, please report.
	//
	#[ error( "Failed to downcast: {ctx}. This indicates an error in thespis_remote, please report at https://github.com/thespis-rs/thespis_remote/issues with a reproducable example and/or a backtrace if possible." ) ]
	//
	Downcast
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Maximum message size exceeded.
	//
	#[ error( "{}", self.clone().remote_err() ) ]
	//
	MessageSizeExceeded
	{
		/// The context in which the error happened.
		//
		context: String ,

		/// The size of the received message.
		//
		size: usize  ,

		/// The maximum allowed message size.
		//
		max_size: usize  ,
	},

	/// Failed to deserialize an Actor message. The message data will be dropped and the remote will be notified of the error.
	/// The connection shall remain functional.
	//
	#[ error( "Failed to deserialize an Actor message{ctx}" ) ]
	//
	Deserialize
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Failed to deserialize incoming data. The connection will be closed because the stream integrity can no longer be assumed.
	//
	#[ error( "Failed to deserialize incoming data. The connection will be closed because the stream integrity can no longer be assumed{ctx}" ) ]
	//
	DeserializeWireFormat
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Failed to deserialize.
	//
	#[ error( "Failed to serialize{ctx}" ) ]
	//
	Serialize
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Cannot deliver message to unknown service.
	//
	#[ error( "Cannot deliver message to unknown service.{ctx}" ) ]
	//
	UnknownService
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// No handler has been set for this service.
	/// If you use the provided ServiceMap implementations, you should only see this if you
	/// use a closure with RelayMap and it returns `None`, because otherwise they don't
	/// advertise services for which they haven't got a handler.
	//
	#[ error( "No handler has been set for this service{ctx}" ) ]
	//
	NoHandler
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Cannot deliver because the handling actor is no longer running.
	//
	#[ error( "Cannot deliver because the handling actor is no longer running.{ctx}" ) ]
	//
	HandlerDead
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Failed to spawn a task.
	//
	#[ error( "Spawning a task failed.{ctx}" ) ]
	//
	Spawn
	{
		/// The contex in which the error happened.
		//
		ctx: ErrorContext
	},

	/// Failed to relay a request because the connection to the relay has been closed.
	//
	#[ error( "Failed to relay a request because the connection to the relay has been closed. context.{ctx} relay_id: {relay_id}, relay_name: {relay_name:?}" ) ]
	//
	RelayGone
	{
		ctx       : ErrorContext     ,
		relay_id  : usize            ,
		relay_name: Option<Arc<str>> ,
	},


	/// This allows also returning all ThesRemoteErr kinds when returning a ThesRemoteErr. eg. Often
	/// operations from remote will use call and send which give mailbox errors, so it's good to
	/// be able to return those as well as the more remote specific errors that might happen in
	/// the same method.
	//
	#[ error( "ThesRemoteErr in context: {}", _0 ) ]
	//
	ThesErr( ThesErr ),


	/// An io::Error happenend in the underlying network connection.
	//
	#[ error( "Io: {:?}", context ) ]
	//
	Io
	{
		/// The ErrorKind
		//
		context: std::io::ErrorKind
	},
}


impl ThesRemoteErr
{
	// Produce a display string suitable for sending errors to a connected client. This omit's
	// process specific information like actor id's.
	//
	pub fn remote_err( self ) -> String
	{
		match self
		{
			Self::MessageSizeExceeded{ context, size, max_size } =>
			{
				format!( "Maximum message size exceeded: context: {}, actual: {} bytes, allowed: {} bytes." , &context, &size, &max_size )
			}

			Self::Deserialize{ mut ctx } =>
			{
				ctx.peer_id   = None;
				ctx.peer_name = None;

				format!( "Could not deserialize your message{}", &ctx )
			}

			_ => { unreachable!() }
		}
	}
}



impl From<ThesErr> for ThesRemoteErr
{
	fn from( cause: ThesErr ) -> ThesRemoteErr
	{
		ThesRemoteErr::ThesErr( cause )
	}
}



impl From< std::io::Error > for ThesRemoteErr
{
	fn from( inner: std::io::Error ) -> ThesRemoteErr
	{
		ThesRemoteErr::Io{ context: inner.kind() }
	}
}



#[ derive( Debug, Clone, PartialEq, Eq ) ]
//
pub struct ErrorContext
{
	pub context  : Option< String    > ,
	pub peer_id  : Option< usize     > ,
	pub peer_name: Option< Arc<str>  > ,
	pub sid      : Option< ServiceID > ,
	pub cid      : Option< ConnID    > ,
}


impl fmt::Display for ErrorContext
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


impl Default for ErrorContext
{
	fn default() -> Self
	{
		ErrorContext
		{
			context  : None ,
			peer_id  : None ,
			peer_name: None ,
			sid      : None ,
			cid      : None ,
		}
	}
}
