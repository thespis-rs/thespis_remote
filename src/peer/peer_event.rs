use crate::{ PeerErr, ConnectionError };


/// Events that can happen during the lifecycle of the peer. Use the [`observe`] method to subscribe to events.
///
/// When you see either `Closed` or `ClosedByRemote`, the connection is lost and you should drop all
/// addresses/recipients you hold for this peer, so it can be dropped. You can no longer send messages
/// over this peer after these events.
//
#[ derive( Debug, Clone, PartialEq, Eq ) ]
//
#[ allow(variant_size_differences) ]
//
pub enum PeerEvent
{
	/// The connection is closed. It can no longer be used.
	//
	Closed,

	/// The remote endpoint closed the connection. It can no longer be used.
	//
	ClosedByRemote,

	/// A remote endpoint to which we relayed messages is no longer reachable.
	//
	RelayDisappeared(usize),

	/// An error happened while handling messages on the connection. These are
	/// generally errors that cannot be bubbled up because they are triggered when
	/// handling incoming messages, and thus are not triggered by a method call from
	/// client code. They are returned here out of band.
	//
	Error( PeerErr ),

	/// The remote endpoint signals that they encountered an error while handling one of
	/// our messages.
	//
	RemoteError( ConnectionError ),
}

