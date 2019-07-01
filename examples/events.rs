#![ feature( async_await, box_syntax ) ]

mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// When errors happen on processing incoming messages over the network, we can't pass up a `Result` in the
// call stack.
//
// Instead a peer is observable as it implements the `Observable` trait from the [pharos](https://crates.io/crates/pharos)
// In this example the `connect_to_tcp` from `common/common.rs` calls `observe()` and gives us a stream of `PeerEvent`.
//
// All events are:
// ```
// pub enum PeerEvent
// {
//    Closed                         ,
//    ClosedByRemote                 ,
//    RelayDisappeared(usize)        ,
//    Error      ( ConnectionError ) ,
//    RemoteError( ConnectionError ) ,
// }
//
// pub enum ConnectionError
// {
//    /// An error deserializing the incoming message. This means the stream might be corrupt,
//    /// so the connection will be closed.
//    //
//    Deserialize,
//
//    /// An error happend when trying to serialize a response.
//    //
//    Serialize,
//
//    /// Your request could not be processed. This might mean spawning a future failed,
//    /// downcasting a Receiver failed or other errors that are clearly not the fault
//    /// of the remote peer.
//    //
//    InternalServerError,
//
//    /// Warn our remote peer that we are no longer providing this service.
//    /// The data is the sid for which we stop providing. This can happen if a relay goes down.
//    /// This means that any further calls to the peer for this service will return
//    /// ConnectionError::ServiceUnknown
//    //
//    ServiceGone(Vec<u8>),
//
//    /// Sending out your message on the connection to the relayed peer failed. If this is
//    /// a permanent failure, eg. ConnectionClosed, you should also get RelayGone errors
//    /// for all the services that are lost.
//    //
//    FailedToRelay(Vec<u8>),
//
//    /// Whilst waiting for the response to your call, the connection to the relay was lost.
//    /// You should also get RelayGone errors  for all the services that are lost.
//    //
//    LostRelayBeforeResponse,
//
//    /// We don't provide this service. Data is sid.
//    //
//    UnknownService(Vec<u8>),
//
//    /// The codec is not valid for the operation.
//    /// The data is the codec passsed in, in serialized form.
//    //
//    UnsupportedCodec(Vec<u8>)
// }
// ```
//
fn main()
{
	let nodea = async
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );

		// register Sum with peer as handler for Add and Show
		//
		let mut sm = remotes::Services::new();

		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );

		// get a framed connection
		//
		let _ = listen_tcp( "127.0.0.1:20003", sm ).await;
	};


	let nodeb = async
	{
		let (mut peera, mut peera_evts)  = connect_to_tcp( "127.0.0.1:20003" ).await;

		// Close the connection and check the event
		//
		peera.send( peer::CloseConnection{ remote: false } ).await.expect( "Send CloseConnection" );

		let next_event = peera_evts.next().await.unwrap();

		println!( "The event from peer (should be “Closed”: {:?}", next_event );

		assert_eq!( PeerEvent::Closed, next_event );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	rt::spawn( nodea  ).expect( "Spawn peera"  );
	rt::spawn( nodeb  ).expect( "Spawn peerb"  );

	rt::run();
}
