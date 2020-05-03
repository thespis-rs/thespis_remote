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
	#[ error( "Maximum message size exceeded: context: {context}, actual: {size} bytes, allowed: {max_size} bytes." ) ]
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
	/// We don't want to use source here because io::Error is not clone, but
	/// for some people having backtraces might be important?
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


impl From< std::io::Error > for WireErr
{
	fn from( inner: std::io::Error ) -> WireErr
	{
		WireErr::Io{ kind: inner.kind() }
	}
}

