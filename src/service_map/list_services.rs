use crate::{ import::*, * };


///
//
#[ derive( Debug ) ]
//
pub struct ListServices;

impl Message for ListServices
{
	/// We do not await the receiver in the async handle method below, since we don't want
	/// to hang the peer whilst waiting for the response. That's why we return a channel.
	//
	type Return = Vec<ServiceID>;
}
