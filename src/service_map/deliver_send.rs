use crate::{ import::*, * };


///
//
#[ derive( Debug ) ]
//
pub struct DeliverSend
{
	pub mesg: WireFormat,
	pub peer: Addr<Peer>,
}

impl Message for DeliverSend
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = ();
}

impl DeliverSend
{
	/// Create a new DeliverSend.
	//
	pub fn new( mesg: WireFormat, peer: Addr<Peer> ) -> Self
	{
		Self{ mesg, peer }
	}
}
