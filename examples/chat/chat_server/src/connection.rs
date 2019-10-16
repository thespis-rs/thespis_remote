use crate::import::*;

pub struct Connection
{
	// tx: UnboundedSender<ServerMsg> ,
	pub peer_addr: SocketAddr,
	pub msgs     : FramedRead <WsStream<WarpWebSocket>, Codec<ClientMsg, ServerMsg>>,
	pub out      : FramedWrite<WsStream<WarpWebSocket>, Codec<ClientMsg, ServerMsg>>,

}


impl Connection
{
	pub fn new( socket: WarpWebSocket, peer_addr: SocketAddr ) -> Self
	{
		let ws_stream = WsStream::new( socket );

		info!( "Incoming connection from: {:?}", peer_addr );

		let (mut tx, rx)    = unbounded();
		let (out, mut msgs) = Framed::new( ws_stream, Codec::new() ).split();

		Self
		{
			peer_addr,
			out,
			msgs,
		}
	}
}
