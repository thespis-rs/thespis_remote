use crate::import::*;


/// Errors that can happen in thespis_impl.
//
#[ derive( Debug, Error ) ]
//
pub enum ThesRemoteErr
{
	/// Cannot use peer after the connection is closed.
	//
	#[ error( "ConnectionClosed: Cannot use peer after the connection is closed, operation: {}", _0 ) ]
	//
	ConnectionClosed( String ),

	/// An error happened when a remote tried to process your message.
	//
	#[ error( "Connection Error: A remote could not process a message we sent it: {}", _0 ) ]
	//
	Connection( String ),

	/// Failed to downcast.
	//
	#[ error( "Failed to downcast: {}", _0 ) ]
	//
	Downcast( String ),

	/// Maximum message size exceeded.
	//
	#[ error( "Maximum message size exceeded: {}", _0 ) ]
	//
	MessageSizeExceeded( String ),

	/// Failed to deserialize.
	//
	#[ error( "Deserialize: Failed to deserialize: {}", _0 ) ]
	//
	Deserialize( String ),

	/// Failed to serialize.
	//
	#[ error( "Serialize: Failed to serialize: {}", _0 ) ]
	//
	Serialize( String ),

	/// Cannot deliver message to unknown service.
	//
	#[ error( "Cannot deliver message to unknown service: {}", _0 ) ]
	//
	UnknownService( String ),

	/// Tokio codec requires that we implement From TokioIoError for the error type of the codec.
	//  TODO: integrate the tokio IO error in here.
	//
	#[ error( "TokioIoError: Tokio IO Error" ) ]
	//
	TokioIoError,

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



impl From<ThesErr> for ThesRemoteErr
{
	fn from( cause: ThesErr ) -> ThesRemoteErr
	{
		ThesRemoteErr::ThesErr( cause )
	}
}


// /// Tokio codec requires an error type which impl this from and our peer impl
// /// requires a sink that has error type ThesRemoteErr, so here we go:
// //
// #[ cfg( feature = "tokio" ) ]
// //
// impl From<tokio::io::Error> for ThesRemoteErr
// {
// 	fn from( e: tokio::io::Error ) -> ThesRemoteErr
// 	{
// 		ThesRemoteErr::TokioIoError
// 	}
// }


impl From< std::io::Error > for ThesRemoteErr
{
	fn from( inner: std::io::Error ) -> ThesRemoteErr
	{
		ThesRemoteErr::Io{ context: inner.kind() }
	}
}
