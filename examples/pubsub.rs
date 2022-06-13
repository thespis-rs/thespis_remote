//! This example shows the pubsub pattern in action with thespis-remote.
//! We will have Alice and Bob listening to message from Carol.
//!
//! Note that PubSub is a specific for relays. You can of course have a specific
//! node that clients can subscribe to and then you send out messages to all of
//! them. However this is a system to take care of that automatically when you are
//! relaying.
//!
//! It has the same function as a relaymap, but it will clone the message and
//! send it to all subscribers. It also has a channel based API where you can just
//! send in addresses to subscribe and id's to unsubscribe.
//!
//! The order of delivery is not guaranteed.
//
use
{
	async_executors     :: { AsyncStd, SpawnHandleExt } ,
	futures_ringbuf     :: { Endpoint                 } ,
	thespis             :: { *                        } ,
	thespis_impl        :: { *                        } ,
	thespis_remote      :: { *, service_map           } ,
	tracing             :: { trace, info              } ,
	serde               :: { Serialize, Deserialize   } ,
	std                 :: { sync::Arc 		           } ,
	futures             :: { future::FutureExt        } ,
};



#[ derive( Actor, Debug, Clone, Copy ) ] struct Sum
{
	count: u64,
	name : &'static str,
}


#[ derive( Serialize, Deserialize, Debug, Clone, Copy ) ] struct Add( u64 );

impl Message for Add  { type Return = ();  }


// Basic handler will just show us on the command line what has been received.
//
impl Handler< Add > for Sum
{
	fn handle( &mut self, msg: Add ) -> Return<'_, ()> { Box::pin( async move
	{
		self.count += msg.0;

		info!( "{}: count: {}", self.name, self.count );
	})}
}




// Which services do we expose:
//
service_map!
(
	namespace  : remote    ;
	wire_format: CborWF    ;
	services   : Add,      ;
);


// our subscribers.
//
async fn bob  ( conn: Endpoint ) { subscriber( conn, "Bob"   ).await; }
async fn alice( conn: Endpoint ) { subscriber( conn, "Alice" ).await; }



async fn subscriber( conn: Endpoint, name: &'static str )
{
	let addr_handler = Addr::builder()

		.name( format!( "{}-Sum", name ) )
		.spawn( Sum { count: 0, name }, &AsyncStd )
		.expect( "spawn actor mailbox" )
	;

	// Create peer with AsyncRead/AsyncWrite
	//
	let (mut peer, peer_mb, _) = CborWF::create_peer( name, conn, 1024, 1024, AsyncStd, None, None ).expect( "create subscriber peer" );

	// Register Sum with peer as handler for Add and Show
	//
	let mut sm = remote::Services::new();

	sm.register_handler::<Add >( Box::new( addr_handler.clone() ) );

	// Register service map with peer
	//
	peer.register_services( Arc::new( sm ) );

	// Start the peer actor
	//
	let peer_handle = AsyncStd.spawn_handle( peer_mb.start( peer ).map(|_|()) ).expect( "start mailbox of Peer" );


	// Wait for the connection to close, which will automatically stop the peer.
	//
	peer_handle.await;

	trace!( "End of {}", name );
}


// Carol will be our sender.
//
async fn carol( conn: Endpoint )
{
	let (relay_peer, relay_mb, mut relay_addr) = CborWF::create_peer( "carol_to_relay", conn, 1024, 1024, AsyncStd, None, None )
		.expect( "spawn peer" )
	;

	let mut addr = remote::RemoteAddr::new( relay_addr.clone() );

	let relay_handle = AsyncStd.spawn_handle( relay_mb.start(relay_peer).map(|_|()) ).expect( "start mailbox of Peer" );

	// Pubsub only works with sends.
	//
	addr.send( Add(1) ).await.expect( "send to relay" );
	addr.send( Add(2) ).await.expect( "send to relay" );
	addr.send( Add(3) ).await.expect( "send to relay" );

	relay_addr.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await
		.expect( "close connection to relay" )
	;

	relay_handle.await;
}


async fn relay( to_alice: Endpoint, to_bob: Endpoint, to_carol: Endpoint )
{
	let (alice_peer, alice_mb, mut alice_addr ) = CborWF::create_peer( "to_alice", to_alice, 1024, 1024, AsyncStd, None, None )
		.expect( "spawn peer" )
	;

	let (bob_peer  , bob_mb  , mut bob_addr   ) = CborWF::create_peer( "to_bob"  , to_bob  , 1024, 1024, AsyncStd, None, None )
		.expect( "spawn peer" )
	;

	let (mut carol_peer, carol_mb, _carol_addr) = CborWF::create_peer( "to_carol", to_carol, 1024, 1024, AsyncStd, None, None )
		.expect( "spawn peer" )
	;


	// Get the service id
	//
	let    add = <Add  as remote::Service>::sid();
	let mut ps = PubSub::new( vec![ add ] );

	ps.subscribe( Box::new( alice_addr.clone() ) );
	ps.subscribe( Box::new( bob_addr.clone()   ) );

	carol_peer.register_services( Arc::new(ps) );

	let alice_handle = AsyncStd.spawn_handle( alice_mb.start(alice_peer).map(|_|()) ).expect( "start mailbox of Peer" );
	let bob_handle   = AsyncStd.spawn_handle( bob_mb  .start(bob_peer  ).map(|_|()) ).expect( "start mailbox of Peer" );
	let carol_handle = AsyncStd.spawn_handle( carol_mb.start(carol_peer).map(|_|()) ).expect( "start mailbox of Peer" );

	// When carol closes the connection, we will propagate that close to bob and alice.
	//
	carol_handle.await;

	bob_addr  .call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );
	alice_addr.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );

	futures::join!( alice_handle, bob_handle );
}



#[ async_std::main ]
//
async fn main()
{
	tracing_subscriber::fmt::Subscriber::builder()

		// .with_timer( tracing_subscriber::fmt::time::ChronoLocal::rfc3339() )
		// .json()
	   .with_env_filter( "info" )
	   .init()
	;


	let (alice_relay, relay_alice) = Endpoint::pair( 64, 64 );
	let (bob_relay  , relay_bob  ) = Endpoint::pair( 64, 64 );
	let (carol_relay, relay_carol) = Endpoint::pair( 64, 64 );

	let relay = relay( relay_alice, relay_bob, relay_carol );
	let alice = alice( alice_relay );
	let bob   = bob  ( bob_relay   );
	let carol = carol( carol_relay );

	futures::join!( relay, alice, bob, carol );
}
