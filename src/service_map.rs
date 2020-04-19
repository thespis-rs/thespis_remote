use crate::{ import::* } ;

mod deliver_call;
mod deliver_send;
mod list_services;

pub use
{
	deliver_call::*,
	deliver_send::*,
	list_services::*,
};

pub trait ServiceMap:

	Address< ListServices, Error=ThesErr > +
	Address< DeliverSend , Error=ThesErr > +
	Address< DeliverCall , Error=ThesErr > +
	Send

{}


impl<T> ServiceMap for T

	where T:

		Address< ListServices, Error=ThesErr > +
		Address< DeliverSend , Error=ThesErr > +
		Address< DeliverCall , Error=ThesErr > +
		Send

{}
