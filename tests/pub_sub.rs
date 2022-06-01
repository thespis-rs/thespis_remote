//! Test PubSub:
//!
//! ✔ Basic usage
//! ✔ Use subscribe and unsubscribe at runtime.
//
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

		let (mut to_relay, _)  = peer_connect( ab, exec, "provider_to_relay" ).await;

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
		let exec     = AsyncStd.instrument( span_relay );
		let services = vec![<Add as remotes::Service>::sid()];
		let delay    = Some( Duration::from_millis(10) );

		// create peer with stream/sink
		//
		let (peer_c, peer_mb_c, mut peer_addr_c) = CborWF::create_peer( "relay_to_consumer_c", bc, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_c" );
		let (peer_d, peer_mb_d, mut peer_addr_d) = CborWF::create_peer( "relay_to_consumer_d", bd, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_d" );
		let (peer_e, peer_mb_e, mut peer_addr_e) = CborWF::create_peer( "relay_to_consumer_e", be, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_e" );

		let (mut peer_a, peer_mb_a, _) = CborWF::create_peer( "relay_to_provider", ba, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_a" );

		let mut pubsub = PubSub::new( services );

		pubsub.subscribe( peer_addr_c.clone_box() );
		pubsub.subscribe( peer_addr_d.clone_box() );
		pubsub.subscribe( peer_addr_e.clone_box() );

		peer_a.register_services( Arc::new( pubsub ) );

		let  peer_a_handle = AsyncStd.spawn_handle( peer_mb_a.start(peer_a) ).expect( "Start mb" );
		let _peer_c_handle = AsyncStd.spawn_handle( peer_mb_c.start(peer_c) ).expect( "Start mb" );
		let _peer_d_handle = AsyncStd.spawn_handle( peer_mb_d.start(peer_d) ).expect( "Start mb" );
		let _peer_e_handle = AsyncStd.spawn_handle( peer_mb_e.start(peer_e) ).expect( "Start mb" );

		peer_a_handle.await;

		warn!( "Closing connection from relay to consumers" );
		peer_addr_c.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_c" );
		peer_addr_d.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_d" );
		peer_addr_e.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_e" );

		warn!( "relays end" );

	}.instrument( span_relay2 );


	let consumer_c = consumer( "task_c", cb, 15, AsyncStd.instrument( span_c.clone() ) ).instrument( span_c );
	let consumer_d = consumer( "task_d", db, 15, AsyncStd.instrument( span_d.clone() ) ).instrument( span_d );
	let consumer_e = consumer( "task_e", eb, 15, AsyncStd.instrument( span_e.clone() ) ).instrument( span_e );

	let relays     = relays  ;
	let provider   = provider.instrument( tracing::info_span!("task_provider") );


	let mut futs = futures::stream::FuturesUnordered::new();

	futs.push( AsyncStd.spawn_handle( consumer_c ).unwrap() );
	futs.push( AsyncStd.spawn_handle( consumer_d ).unwrap() );
	futs.push( AsyncStd.spawn_handle( consumer_e ).unwrap() );
	futs.push( AsyncStd.spawn_handle( relays     ).unwrap() );
	futs.push( AsyncStd.spawn_handle( provider   ).unwrap() );

	while futs.next().await.is_some() {}
}


async fn consumer( name: &str, endpoint: Endpoint, expect: i64, exec: tracing_futures::Instrumented<AsyncStd> )
{
	// Create mailbox for our handler
	//
	debug!( "start mailbox for handler on {}", name );
	let mut addr_handler = Addr::builder().spawn( Sum(0), &exec ).expect( "spawn actor mailbox" );


	// register Sum with peer as handler for Add and Show
	//
	let mut sm = remotes::Services::new();

	sm.register_handler::<Add >( addr_handler.clone_box() );


	// get a framed connection
	//
	debug!( "start mailbox for consumer" );
	let (_, _peer_evts, handle) = peer_listen( endpoint, Arc::new( sm ), exec, name ).await;

	handle.await;

	assert_eq!( expect, addr_handler.call(Show).await.expect( "Call failed"), "{}", &name );

	trace!( "End of consumer {}", name );
}



// Some hypothetical steps in our flow.
//
#[ derive( Debug, Clone, PartialEq, Eq )]
//
enum Step
{
	Start,
   Send1,
   Send2,
   Send3,
   Sub2 ,
   Sub3 ,
}


// Test subscribe/unsubscribe at runtime.
// TODO: this test is lousy. It uses arbitrary timers to slow things down. Need a way to make it
// deterministic.
//
#[async_std::test]
//
async fn pubsub_rt()
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

	let steps  = async_progress::Progress::new( Step::Start );
	let steps2 = steps.clone();

	let send1 = steps.once( Step::Send1 );
	let send2 = steps.once( Step::Send2 );
	let send3 = steps.once( Step::Send3 );
	let sub2  = steps.once( Step::Sub2  );
	let sub3  = steps.once( Step::Sub3  );


	let provider = async move
	{
		let exec = AsyncStd.instrument( span_provider );

		debug!( "start mailbox for provider_to_relay" );

		let (mut to_relay, _)  = peer_connect( ab, exec, "provider_to_relay" ).await;

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( to_relay.clone() );


		send1.await;

			assert_eq!( Ok(()), addr.send( Add(5) ).await );

		steps2.set_state( Step::Sub2 ).await;
		send2.await;

			assert_eq!( Ok(()), addr.send( Add(5) ).await );

		steps2.set_state( Step::Sub3 ).await;
		send3.await;

			assert_eq!( Ok(()), addr.send( Add(5) ).await );


		warn!( "provider end, telling relay to close connection" );

		to_relay.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to relay" );

		warn!( "provider end, relay processed CloseConnection" );

	}.instrument( span_provider2 );


	let relays = async move
	{
		let exec     = AsyncStd.instrument( span_relay );
		let services = vec![<Add as remotes::Service>::sid()];
		let delay    = Some( Duration::from_millis(10) );

		// create peer with stream/sink
		//
		let (peer_c, peer_mb_c, mut peer_addr_c) = CborWF::create_peer( "relay_to_consumer_c", bc, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_c" );
		let (peer_d, peer_mb_d, mut peer_addr_d) = CborWF::create_peer( "relay_to_consumer_d", bd, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_d" );
		let (peer_e, peer_mb_e, mut peer_addr_e) = CborWF::create_peer( "relay_to_consumer_e", be, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_e" );

		let (mut peer_a, peer_mb_a, _) = CborWF::create_peer( "relay_to_provider", ba, 1024, 1024, exec.clone(), None, delay ).expect( "spawn peer_a" );


		let mut pubsub       = PubSub::new( services );
		let mut subscriber   = pubsub.rt_subscribe  ( &exec ).expect( "get subsriber"   );
		let mut unsubscriber = pubsub.rt_unsubscribe( &exec ).expect( "get unsubsriber" );


		peer_a.register_services( Arc::new( pubsub ) );


		let  peer_a_handle = AsyncStd.spawn_handle( peer_mb_a.start(peer_a) ).expect( "Start mb" );
		let _peer_c_handle = AsyncStd.spawn_handle( peer_mb_c.start(peer_c) ).expect( "Start mb" );
		let _peer_d_handle = AsyncStd.spawn_handle( peer_mb_d.start(peer_d) ).expect( "Start mb" );
		let _peer_e_handle = AsyncStd.spawn_handle( peer_mb_e.start(peer_e) ).expect( "Start mb" );

			let peer_addr_c2 = peer_addr_c.clone_box();
			subscriber.send( peer_addr_c2 ).await.expect( "send on channel" );

		steps.set_state( Step::Send1 ).await;
		sub2.await;
		futures_timer::Delay::new( Duration::from_millis(1)).await;

			let peer_addr_d2 = peer_addr_d.clone_box();
			subscriber.send( peer_addr_d2 ).await.expect( "send on channel" );

		steps.set_state( Step::Send2 ).await;
		sub3.await;
		futures_timer::Delay::new( Duration::from_millis(1)).await;

			let peer_addr_e2 = peer_addr_e.clone_box();
			subscriber.send( peer_addr_e2 ).await.expect( "send on channel" );
			let id = peer_addr_e.id();
			unsubscriber.send( id ).await.expect( "send on channel" );

		steps.set_state( Step::Send3 ).await;


		peer_a_handle.await;

		warn!( "Closing connection from relay to consumers" );
		peer_addr_c.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_c" );
		peer_addr_d.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_d" );
		peer_addr_e.call( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection relay_e" );

		warn!( "relays end" );

	}.instrument( span_relay2 );


	let consumer_c = consumer( "task_c", cb, 15, AsyncStd.instrument( span_c.clone() ) ).instrument( span_c );
	let consumer_d = consumer( "task_d", db, 10, AsyncStd.instrument( span_d.clone() ) ).instrument( span_d );
	let consumer_e = consumer( "task_e", eb, 0, AsyncStd.instrument( span_e.clone() ) ).instrument( span_e );

	let relays     = relays  ;
	let provider   = provider.instrument( tracing::info_span!("task_provider") );


	let mut futs = futures::stream::FuturesUnordered::new();

	futs.push( AsyncStd.spawn_handle( consumer_c ).unwrap() );
	futs.push( AsyncStd.spawn_handle( consumer_d ).unwrap() );
	futs.push( AsyncStd.spawn_handle( consumer_e ).unwrap() );
	futs.push( AsyncStd.spawn_handle( relays     ).unwrap() );
	futs.push( AsyncStd.spawn_handle( provider   ).unwrap() );

	while futs.next().await.is_some() {}
}


