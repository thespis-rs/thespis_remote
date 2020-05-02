/// Type of message.
//
#[ derive(Debug) ]
//
pub(crate) enum WireType
{
	ConnectionError,
	IncomingSend,
	IncomingCall,
	CallResponse,
}
