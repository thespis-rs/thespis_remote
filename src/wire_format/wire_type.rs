/// Type of message.
//
#[ derive(Debug, Copy, Clone) ]
//
pub enum WireType
{
	ConnectionError,
	IncomingSend,
	IncomingCall,
	CallResponse,
}
