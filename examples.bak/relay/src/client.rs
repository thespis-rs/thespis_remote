mod common;

use common::{ import::*, * };


#[ async_std::main ]
//
async fn main()
{
	flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	debug!( "start mailbox for client_to_relay" );

	let tcp = TcpStream::connect( RELAY ).await.expect( "Connect to relay" );

	let (mut to_relay, evts) = peer_connect( tcp, AsyncStd, "client_to_relay" );

	// We don't use these in this example, but if we use `let _` they will only get
	// dropped at the end of this async block, which is the end of the process.
	// In the meanwhile the peer will send event's to this object, even though
	// it's not being used.
	//
	// In a real app you should almost always use the events, because the errors
	// are returned out of bounds by means of these events.
	//
	drop( evts );

	// Call the service and receive the response
	//
	let mut addr = remotes::RemoteAddr::new( to_relay.clone() );

	let resp = addr.call( Add(5) ).await.expect( "Call failed" );
	assert_eq!( (), resp );

	addr.send( Add(5) ).await.expect( "Send failed" );

	let resp = addr.call( Show ).await.expect( "Call failed" );
	assert_eq!( 10, resp );

	to_relay.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );
}
