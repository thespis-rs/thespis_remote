use crate::{ import::*, PeerErr } ;

mod unique_id  ;
mod conn_id    ;
mod service_id ;
mod wire_err   ;
mod wire_type  ;

pub use
{
	service_id :: * ,
	conn_id    :: * ,
	wire_err   :: * ,
};

pub(crate) use wire_type::WireType;

/// Trait holding the required functionality to function as a WireFormat for thespis_remote.
//
pub trait WireFormat : Message< Return = Result<(), PeerErr> > + From< (ServiceID, ConnID, Bytes) >
{
	/// The service id of this message. When coming in over the wire, this identifies
	/// which service you are calling. A ServiceID should be unique for a given service.
	/// The reference implementation combines a unique type id with a namespace so that
	/// several processes can accept the same type of service under a unique name each.
	//
	fn service ( &self ) -> ServiceID;

	/// The connection id. This is used to match responses to outgoing calls.
	//
	fn conn_id( &self ) -> ConnID;

	/// The serialized payload message.
	//
	fn mesg( &self ) -> Bytes;

	/// The total length of the WireFormat in bytes (header+payload).
	//
	fn len( &self ) -> usize;


	fn kind( &self ) -> WireType
	{
		match self.service()
		{
			x if x.is_null() => WireType::ConnectionError ,
			x if x.is_full() => WireType::CallResponse    ,

			_ =>
			{
				match self.conn_id()
				{
					x if x.is_null() => WireType::IncomingSend ,
					_                => WireType::IncomingCall ,
				}
			}
		}
	}
}





