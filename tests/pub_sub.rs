
mod common;

use common::*                       ;
use common::import::{ *, assert_eq };

use tracing_futures::Instrument;


// Test relaying messages
//
#[async_std::test]
//
async fn pubsub()
{
	// tracing_subscriber::fmt::Subscriber::builder()

	// 	// .with_timer( tracing_subscriber::fmt::time::ChronoLocal::rfc3339() )
	// 	.json()
	//    .with_max_level(tracing::Level::TRACE)
	//    .init()
	// ;

	let span_c         = tracing::info_span!( "task_c"       );
	let span_d         = tracing::info_span!( "task_d"       );
	let span_e         = tracing::info_span!( "task_e"       );
	let span_relay     = tracing::info_span!( "task_relay"    );
	let span_provider  = tracing::info_span!( "task_provider" );
	let span_relay2    = span_relay.clone();
	let span_provider2 = span_provider.clone();

	let (ab, ba) = Endpoint::pair( 64, 64 );
	let (bc, cb) = Endpoint::pair( 64, 64 );
	let (bd, db) = Endpoint::pair( 64, 64 );
	let (be, eb) = Endpoint::pair( 64, 64 );


	let provider = async move
	{
		let exec = AsyncStd.instrument( span_provider );

		debug!( "start mailbox for provider_to_relay" );

		let (mut to_relay, _)  = peer_connect( ab, exec, "provider_to_relay" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_relay.clone() );

		assert_eq!( Ok(()), addr.send( Add(5) ).await );
		assert_eq!( Ok(()), addr.send( Add(5) ).await );
		assert_eq!( Ok(()), addr.send( Add(5) ).await );


		warn!( "provider end, telling relay to close connection" );

		to_relay.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );

		warn!( "provider end, relay processed CloseConnection" );

	}.instrument( span_provider2 );


	let relays = async move
	{
		let exec = AsyncStd.instrument( span_relay );


		let services = vec![<Add as remotes::Service>::sid()];

		// Create mailbox for peer to consumer_c
		//
		let (peer_addr_a, peer_mb_a) = Addr::builder().name( "relay_to_provider".into() ).build();

		let (mut peer_addr_c, peer_mb_c) = Addr::builder().name( "relay_to_consumer_c".into() ).build();
		let (mut peer_addr_d, peer_mb_d) = Addr::builder().name( "relay_to_consumer_d".into() ).build();
		let (mut peer_addr_e, peer_mb_e) = Addr::builder().name( "relay_to_consumer_e".into() ).build();

		let delay = Some( Duration::from_millis(10) );

		// create peer with stream/sink
		//
		let peer_c = Peer::from_async_read( peer_addr_c.clone(), bc, 1024, exec.clone(), None, delay.clone() ).expect( "spawn peer_c" );
		let peer_d = Peer::from_async_read( peer_addr_d.clone(), bd, 1024, exec.clone(), None, delay.clone() ).expect( "spawn peer_d" );
		let peer_e = Peer::from_async_read( peer_addr_e.clone(), be, 1024, exec.clone(), None, delay.clone() ).expect( "spawn peer_e" );

		let mut peer_a = Peer::from_async_read( peer_addr_a, ba, 1024, exec.clone(), None, delay ).expect( "spawn peer" );

		let mut pubsub = PubSub::new( services );

		pubsub.subscribe( peer_addr_c.clone_box() );
		pubsub.subscribe( peer_addr_d.clone_box() );
		pubsub.subscribe( peer_addr_e.clone_box() );

		peer_a.register_services( Arc::new( pubsub ) );

		let  peer_a_handle = peer_mb_a.start_handle( peer_a, &exec ).expect( "spawn mb" );
		let _peer_c_handle = peer_mb_c.start_handle( peer_c, &exec ).expect( "spawn mb" );
		let _peer_d_handle = peer_mb_d.start_handle( peer_d, &exec ).expect( "spawn mb" );
		let _peer_e_handle = peer_mb_e.start_handle( peer_e, &exec ).expect( "spawn mb" );


		peer_a_handle.await;

		warn!( "Closing connection from relay to consumers" );
		peer_addr_c.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_c" );
		peer_addr_d.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_d" );
		peer_addr_e.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_e" );

		warn!( "relays end" );

	}.instrument( span_relay2 );




	let consumer_c = consumer( "task_c", cb, AsyncStd.instrument( span_c.clone() ) ).instrument( span_c );
	let consumer_d = consumer( "task_d", db, AsyncStd.instrument( span_d.clone() ) ).instrument( span_d );
	let consumer_e = consumer( "task_e", eb, AsyncStd.instrument( span_e.clone() ) ).instrument( span_e );

	let relays     = relays  ;
	let provider   = provider.instrument( tracing::info_span!("task_provider") );


	let mut futs = futures::stream::FuturesUnordered::new();

	futs.push( AsyncStd.spawn_handle( consumer_c ).unwrap() );
	futs.push( AsyncStd.spawn_handle( consumer_d ).unwrap() );
	futs.push( AsyncStd.spawn_handle( consumer_e ).unwrap() );
	futs.push( AsyncStd.spawn_handle( relays     ).unwrap() );
	futs.push( AsyncStd.spawn_handle( provider   ).unwrap() );

	while let Some(_) = futs.next().await {}
}


async fn consumer( name: &str, endpoint: Endpoint, exec: tracing_futures::Instrumented<AsyncStd> )
{
	// Create mailbox for our handler
	//
	debug!( "start mailbox for handler on {}", name );
	let mut addr_handler = Addr::builder().start( Sum(0), &exec ).expect( "spawn actor mailbox" );


	// register Sum with peer as handler for Add and Show
	//
	let mut sm = remotes::Services::new();

	sm.register_handler::<Add >( addr_handler.clone_box() );


	// get a framed connection
	//
	debug!( "start mailbox for consumer" );
	let (peer_addr, _peer_evts, handle) = peer_listen( endpoint, Arc::new( sm ), exec, name );

	// this way the only peer address is in the peer itself, so we don't keep it alive after the connection ends.
	//
	drop( peer_addr );

	handle.await;

	assert_eq!( 15, addr_handler.call(Show).await.expect( "Call failed") );

	trace!( "End of consumer {}", name );
}


