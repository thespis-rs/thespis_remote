//! This example demonstrates both how relaying works in thespis as well as the logging you can expect
//! with tracing.
//!
//! It is a simple app with 3 components:
//! - A server which has an actor that can handle an Add and a Show
//!   message. It will add to a counter and return the counter value when receiving a Show message.
//! - A relay which will expose the Add and Show services, but won't handle them itself. Rather it
//!   will relay the messages to the server.
//! - A client which will use the service, but rather than directly connecting to the server, it will
//!   use the relay.
//!
//! The three components run in the same process as concurrent tasks. They communicate through a mock
//! connection from the futures_ringbuf crate, but it works just the same over TCP.
//!
//! With tracing you can see that we can instrument the executors each of these components use so that
//! we know in which concurrent task log entries originate, even if those tasks need to spawn. With
//! a tool called tracing_prism we can see the logs for each of these components side by side which
//! greatly improves readability as intertwined logs are very hard to read: https://github.com/najamelan/tracing_prism
//!
//! You also get the features from thespis_impl which will make sure that any log message that happens within a handler
//! will have the actor name and id attached to it.
//!
//
mod import
{
	pub(crate) use
	{
		async_executors     :: { AsyncStd, SpawnHandleExt, JoinHandle } ,
		async_nursery       :: { Nursery, NurseExt                    } ,
		futures_ringbuf     :: { Endpoint                             } ,
		thespis             :: { *                                    } ,
		thespis_impl        :: { *                                    } ,
		thespis_remote      :: { *, service_map                       } ,
		tracing             :: { trace, debug, warn, info             } ,
		pharos              :: { Observable, ObserveConfig, Events    } ,
		serde               :: { Serialize, Deserialize               } ,
		std                 :: { sync::Arc 		                       } ,
		futures             :: { future::FutureExt                    } ,
		tracing_futures     :: { Instrument                           } ,
	};
}

use import::*;



#[ derive( Actor, Debug, Clone, Copy ) ] struct Sum( u64 );

#[ derive( Serialize, Deserialize, Debug, Clone, Copy ) ] struct Add( u64 );
#[ derive( Serialize, Deserialize, Debug, Clone, Copy ) ] struct Show;

impl Message for Add  { type Return = ();  }
impl Message for Show { type Return = u64; }



impl Handler< Add > for Sum
{
	fn handle( &mut self, msg: Add ) -> Return<'_, ()> { Box::pin( async move
	{
		trace!( "called sum with: {:?}", msg );

		self.0 += msg.0;

	})}
}


impl Handler< Show > for Sum
{
	fn handle( &mut self, _msg: Show ) -> Return<'_, u64> { Box::pin( async move
	{
		trace!( "called sum with: Show" );

		self.0

	})}
}


// Which services do we expose:
//
service_map!
(
	namespace  : remotes   ;
	wire_format: CborWF    ;
	services   : Add, Show ;
);


#[async_std::main]
//
async fn main()
{
	tracing_subscriber::fmt::Subscriber::builder()

		// .with_timer( tracing_subscriber::fmt::time::ChronoLocal::rfc3339() )
		// .json()
	   .with_env_filter( "trace,polling=warn,async_io=warn,async_std::warn" )
	   .init()
	;


	let (nursery, output) = Nursery::new( AsyncStd );

	let (server_relay, relay_server) = Endpoint::pair( 64, 64 );
	let (relay_client, client_relay) = Endpoint::pair( 64, 64 );

	let server = server( server_relay               ).instrument( tracing::info_span!( "server_span" ) ) ;
	let relays = relay ( relay_server, relay_client ).instrument( tracing::info_span!( "relays_span" ) ) ;
	let client = client( client_relay               ).instrument( tracing::info_span!( "client_span" ) ) ;

	nursery.nurse( server ).expect( "spawn server" );
	nursery.nurse( relays ).expect( "spawn server" );
	nursery.nurse( client ).expect( "spawn server" );

	drop( nursery );

	output.await;
}



async fn server( to_relay: Endpoint )
{
	let exec = AsyncStd.instrument( tracing::info_span!( "server_span" ) );

	// Create mailbox for our handler
	//
	debug!( "start mailbox for handler" );

	let addr_handler = Addr::builder().name( "sum_handler" ).spawn( Sum(0), &exec ).expect( "spawn actor mailbox" );

	// Create peer with stream/sink
	//
	let (mut peer, peer_mb, _) = CborWF::create_peer( "server", to_relay, 1024, 1024, Arc::new( exec.clone() ), None ).expect( "create peer" );

	// Register Sum with peer as handler for Add and Show
	//
	let mut sm = remotes::Services::new();

	sm.register_handler::<Add >( Box::new( addr_handler.clone() ) );
	sm.register_handler::<Show>( Box::new( addr_handler.clone() ) );


	// Register service map with peer
	//
	peer.register_services( Arc::new( sm ) );

	// Start the peer actor
	//
	let peer_handle = exec.spawn_handle( peer_mb.start( peer ).map(|_|()) ).expect( "start mailbox of Peer" );


	// Wait for the connection to close, which will automatically stop the peer.
	//
	peer_handle.await;

	trace!( "End of server" );
}



async fn relay( to_server: Endpoint, to_client: Endpoint )
{
	let exec = AsyncStd.instrument( tracing::info_span!( "relays_span" ) );

	debug!( "start mailbox for relay_to_provider" );

	let (mut server_addr, _evts, server_handle) = peer_connect( to_server, exec.clone(), "relay_to_provider" ).await;

	debug!( "Actor for relay_to_provider is {}", server_addr.id() );

	// create peer with stream/sink + service map
	//
	let (mut client_peer, client_mb, _) = CborWF::create_peer( "relay_to_consumer", to_client, 1024, 1024, exec.clone(), None ).expect( "spawn peer" );

	let add  = <Add  as remotes::Service>::sid();
	let show = <Show as remotes::Service>::sid();

	let services = vec![ add, show ];

	let rm = Arc::new( RelayMap::new( ServiceHandler::Address( Box::new( server_addr.clone() ) ), services ) );

	client_peer.register_services( rm );

	debug!( "start mailbox for relay_to_consumer" );
	client_mb.start( client_peer ).await;
	warn!( "relay finished, closing connection" );

	server_addr.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await

		.expect( "close connection to provider" );

	server_handle.await;

	warn!( "relays end" );
}



async fn client( client_relay: Endpoint )
{
	let exec = AsyncStd.instrument( tracing::info_span!( "client_span" ) );

	debug!( "start mailbox for client_to_relay" );

	let (mut to_relay, evts, relay_handle) = peer_connect( client_relay, exec, "client_to_relay" ).await;

	// We don't use these in this example, but if we use `let _` they will only get
	// dropped at the end of this async block, which is the end of the process.
	// In the meanwhile the peer will send event's to this object, even though
	// it's not being used.
	//
	// In a real app you should almost always use the events, because the errors
	// are returned out of band by means of these events.
	//
	drop( evts );

	// Call the service and receive the response
	// To actually use our remote services, we transform out peer address into a
	// RemoteAddress created for us by the service_map! macro. This address
	// accepts the correct types of messages and will serialize them for us.
	//
	let mut addr = remotes::RemoteAddr::new( to_relay.clone() );

	info!( "call with Add(5)" );
	addr.call( Add(5) ).await.expect( "Call failed" );

	info!( "send with Add(5)" );
	addr.send( Add(5) ).await.expect( "Send failed" );

	info!( "call with Show" );
	let resp = addr.call( Show ).await.expect( "Call failed" );
	assert_eq!( 10, resp );

	info!( "Close Connection" );
	to_relay.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );

	relay_handle.await;
}



// Keep the boilerplate DRY. This create a peer, spawns it and observes it.
//
async fn peer_connect
(
	socket: Endpoint      ,
	exec  : impl PeerExec ,
	name  : &str          ,
)
	-> (WeakAddr<Peer>, Events<PeerEvent>, JoinHandle<()>)
{
	// create peer with stream/sink + service map
	//
	let (mut peer, peer_mb, peer_addr) = CborWF::create_peer( name, socket, 1024, 1024, exec.clone(), None ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).await.expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	let handle = exec.spawn_handle( peer_mb.start(peer).map(|_|()) ).expect( "start mailbox of Peer" );

	(peer_addr, evts, handle)
}
