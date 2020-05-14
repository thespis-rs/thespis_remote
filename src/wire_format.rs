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
#[ allow(clippy::len_without_is_empty) ]
//
pub trait WireFormat : Message< Return = Result<(), PeerErr> > + Default + Clone + io::Write
{
	/// The service id of this message. When coming in over the wire, this identifies
	/// which service you are calling. A ServiceID should be unique for a given service.
	/// The reference implementation combines a unique type id with a namespace so that
	/// several processes can accept the same type of service under a unique name each.
	//
	fn sid( &self ) -> ServiceID;

	fn set_sid( &mut self, sid: ServiceID );

	/// The connection id. This is used to match responses to outgoing calls.
	//
	fn cid( &self ) -> ConnID;

	fn set_cid( &mut self, cid: ConnID );


	/// The serialized payload message.
	//
	fn mesg( &self ) -> &[u8];

	/// The total length of the WireFormat in bytes (header+payload).
	//
	fn len( &self ) -> u64;

	fn set_len( &mut self, len: u64 );

	/// Make sure there is enough room for the serialized payload to avoid frequent re-allocation.
	//
	fn with_capacity( size: usize ) -> Self;


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





