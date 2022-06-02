use crate::{ import::*, PeerErr, Peer, PeerExec } ;

mod unique_id  ;
mod conn_id    ;
mod service_id ;
mod wire_err   ;
mod wire_type  ;

#[ cfg(any( test, feature="wf_test" )) ] mod testsuite;
#[ cfg(any( test, feature="wf_test" )) ] pub use testsuite::*;

pub use
{
	service_id :: * ,
	conn_id    :: * ,
	wire_err   :: * ,
};

pub(crate) use wire_type::WireType;

/// Trait holding the required functionality to function as a WireFormat for thespis_remote.
//
#[ allow(clippy::len_without_is_empty) ]
//
pub trait WireFormat: Message< Return = Result<(), PeerErr> > + Default + Clone + io::Write
{
	/// Create a Peer from a connection that implements [AsyncRead]/[AsyncWrite]. Most of the parameters
	/// here can be passed on to [Peer::new]. However you must turn the turn the [AsyncRead]/[AsyncWrite]
	/// into a [Stream]/[Sink] over your message type.
	///
	/// *grace_period*: When the remote closes the connection, we could immediately drop all outstanding tasks related to
	/// this peer. This makes sense for a request-response type connection, as it doesn't make sense to
	/// continue using resources processing requests for which we can no longer send the response. However
	/// it is not always desirable. For one way information flow, we might want to finish processing all the
	/// outstanding packets before closing down. This also applies when you send a `CloseConnection` message to
	/// this peer locally.
	///
	/// For implementors: Your codec must also respect the max size for reading and writing messages. That is if the user
	/// attempts to write a message that is too big, your codec should return [WireErr::MessageSizeExceeded].
	/// The same in case your reader detects a message on the network that is too big.
	//
	fn create_peer
	(
		name          : impl AsRef<str>                                      ,
		socket        : impl AsyncRead + AsyncWrite + Unpin + Send + 'static ,
		max_size_read : usize                                                ,
		max_size_write: usize                                                ,
		exec          : impl PeerExec<Self>                                  ,
		bp            : Option< Arc<Semaphore> >                             ,
		grace_period  : Option< Duration >                                   ,
	)

	-> Result< (Peer<Self>, Mailbox<Peer<Self>>, WeakAddr<Peer<Self>>), PeerErr >

		where Self: Sized
	;



	/// The service id of this message. When coming in over the wire, this identifies
	/// which service you are calling. A ServiceID should be unique for a given service.
	/// The reference implementation combines a unique type id with a namespace so that
	/// several processes can accept the same type of service under a unique sid each.
	//
	fn sid( &self ) -> ServiceID;

	/// Set the sid of the message.
	//
	fn set_sid( &mut self, sid: ServiceID ) -> &mut Self;

	/// The connection id. Used for calls to correlate the response to the request.
	//
	fn cid( &self ) -> ConnID;

	/// Set the connection id.
	//
	fn set_cid( &mut self, cid: ConnID ) -> &mut Self;

	/// The serialized payload message. This is the actual actor message to be deserialized and
	/// delivered to the actor.
	//
	fn msg( &self ) -> &[u8];

	/// The total length of the WireFormat in bytes. Generally this is the first field of the
	/// header which allows decoders to allocate the correct amount of buffer to read the rest
	/// of the message.
	//
	fn len( &self ) -> u64;

	/// Make sure there is enough room for the serialized payload to avoid frequent re-allocation.
	//
	fn with_capacity( size: usize ) -> Self;

	/// Deciphers from the sid and cid values what kind of message this is. It distinguishes between
	/// the variants in [`WireType`].
	//
	fn kind( &self ) -> WireType
	{
		match self.sid()
		{
			x if x.is_null() => WireType::ConnectionError ,
			x if x.is_full() => WireType::CallResponse    ,

			_ =>
			{
				match self.cid()
				{
					x if x.is_null() => WireType::IncomingSend ,
					_                => WireType::IncomingCall ,
				}
			}
		}
	}
}





