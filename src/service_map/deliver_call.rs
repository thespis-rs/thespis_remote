use crate::{ import::*, * };


///
//
#[ derive( Debug ) ]
//
pub struct DeliverCall
{
	pub mesg: WireFormat,
	pub peer: Addr<Peer>,
}

impl Message for DeliverCall
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = ();
}

impl DeliverCall
{
	/// Create a new DeliverCall.
	//
	pub fn new( mesg: WireFormat, peer: Addr<Peer> ) -> Self
	{
		Self{ mesg, peer }
	}
}
