use crate::{ import::* };


/// Errors that can happen in thespis_impl.
//
#[ derive( Debug, Clone, PartialEq, Eq ) ]
//
#[ non_exhaustive ]
//
pub enum WireErr
{
	/// Maximum message size exceeded.
	//
	MessageSizeExceeded
	{
		/// The context in which the error happened.
		//
		context: String,

		/// The size of the received message in bytes.
		//
		size: usize,

		/// The maximum allowed message size in bytes.
		//
		max_size: usize,
	},


	/// Failed to deserialize incoming data. The connection will be closed because the stream integrity can no longer be assumed.
	//
	Deserialize
	{
		/// The contex in which the error happened.
		//
		context: String,
	},


	/// An io::Error happenend in the underlying network connection.
	///
	/// We don't want to use source here because io::Error is not clone, but
	/// for some people having backtraces might be important?
	//
	Io
	{
		/// The ErrorKind
		//
		kind: std::io::ErrorKind
	},
}



impl std::error::Error for WireErr {}


impl fmt::Display for WireErr
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match &self
		{
			WireErr::MessageSizeExceeded{ context, size, max_size } =>

				write!( f, "Maximum message size exceeded: context: {}, actual: {} bytes, allowed: {} bytes.", context, size, max_size ),

			WireErr::Deserialize{ context } =>

				write!( f, "Failed to deserialize incoming data. The connection will be closed because the stream integrity can no longer be assumed{}", context ),

			WireErr::Io{ kind } =>

				write!( f, "Io: {:?}", kind ),
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

