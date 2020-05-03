#![ cfg( feature = "futures_codec" ) ]

// Tests:
//
// - ✔ Test error returned by timeout.
// - TODO: Test timeout in relay.
//
mod common;

use
{
	common        :: { *, import::{ *, assert_eq }                             } ,
	std           :: { time::Duration, sync::atomic::{ AtomicUsize, Ordering } } ,
	futures_timer :: { Delay                                                   } ,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[ derive(Actor) ] struct Slow;

impl Handler<Add> for Slow
{
	fn handle( &mut self, _msg: Add ) -> Return<'_, ()> { async move
	{
		Delay::new( Duration::from_millis(100) ).await;

		COUNTER.fetch_add( 1, Ordering::SeqCst );

	}.boxed() }
}



service_map!
(
	namespace: timeouts ;
	services : Add      ;
);



// Test error returned by timeout.
//
#[async_std::test]
//
async fn timeout()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let peera = async move
	{
		// Create mailbox for our handler
		//
		let addr_handler = Addr::builder().start( Slow, &AsyncStd ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let mut sm = timeouts::Services::new();

		// Register our handlers
		//
		sm.register_handler::<Add >( addr_handler.clone_box() );

		// get a framed connection
		//
		let (_, _, handle) = peer_listen( server, Arc::new( sm ), AsyncStd, "peera" );

		handle.await;

		trace!( "end of peera" );
	};


	let peerb = async move
	{
		let name: Arc<str> = "timeout client".into();

		// Create mailbox for peer
		//
		let (mut peera, peer_mb) = Addr::builder().name( name.clone() ).build();

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::from_async_read( peera.clone(), client, 1024, AsyncStd, None ).expect( "spawn peer" );


		// This is the relevant line for this test!
		//
		peer.set_timeout( Duration::from_millis( 10 ) );


		debug!( "start mailbox for [{}] in peerb", name );

		peer_mb.spawn( peer, &AsyncStd ).expect( "start mailbox of Peer" );


		// Call the service and receive the response
		//
		let mut remote_addr = timeouts::RemoteAddr::new( peera.clone() );

		let resp = remote_addr.call( Add(1) ).await;

		assert_matches!( resp, Err( PeerErr::Timeout{..} ) );
		assert_eq!     ( COUNTER.load( Ordering::SeqCst ), 0 );

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	join( peera, peerb ).await;
}

