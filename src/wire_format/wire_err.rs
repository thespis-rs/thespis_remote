use crate::{ import::* };


/// Errors that can happen in thespis_impl.
//
#[ derive( Debug, Error, Clone, PartialEq, Eq ) ]
//
#[ non_exhaustive ]
//
pub enum WireErr
{
	/// Maximum message size exceeded.
	//
	#[ error( "{}", self.clone().remote_err() ) ]
	//
	MessageSizeExceeded
	{
		/// The context in which the error happened.
		//
		context: String,

		/// The size of the received message.
		//
		size: usize,

		/// The maximum allowed message size.
		//
		max_size: usize,
	},


	/// Failed to deserialize incoming data. The connection will be closed because the stream integrity can no longer be assumed.
	//
	#[ error( "Failed to deserialize incoming data. The connection will be closed because the stream integrity can no longer be assumed{context}" ) ]
	//
	Deserialize
	{
		/// The contex in which the error happened.
		//
		context: String,
	},


	/// An io::Error happenend in the underlying network connection.
	///
	/// TODO: stick to source convention? We would lose Clone...
	//
	#[ error( "Io: {:?}", kind ) ]
	//
	Io
	{
		/// The ErrorKind
		//
		kind: std::io::ErrorKind
	},
}


impl WireErr
{
	/// Produce a display string suitable for sending errors to a connected client. This omit's
	/// process specific information like actor id's.
	//
	pub fn remote_err( self ) -> String
	{
		match self
		{
			Self::MessageSizeExceeded{ context, size, max_size } =>
			{
				format!( "Maximum message size exceeded: context: {}, actual: {} bytes, allowed: {} bytes." , &context, &size, &max_size )
			}

			Self::Deserialize{ context } =>
			{
				format!( "Could not deserialize your message, context: {}", &context )
			}

			_ => { unreachable!() }
		}
	}
}


impl From< std::io::Error > for WireErr
{
	fn from( inner: std::io::Error ) -> WireErr
	{
		WireErr::Io{ kind: inner.kind() }
	}
}

