/// Type of message.
//
#[ derive(Debug) ]
//
pub enum WireType
{
	ConnectionError,
	IncomingSend,
	IncomingCall,
	CallResponse,
}
