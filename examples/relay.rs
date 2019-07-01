#![ feature( async_await, box_syntax ) ]

mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// Helper method to create relays
// almost everything specific to relaying is in this method. The main function below is almost the same
// as the basic example.
//
// The next parameter could be called consumer here, but you can actually chain as many relays as you want
// and that's exaclty what the integration tests do.
//
async fn relay( connect: &'static str, listen: &'static str, next: Pin<Box< dyn Future<Output=()> + Send>> )
{
	// Part that connects to the provider
	//
	let (mut provider_addr, provider_evts) = connect_to_tcp( connect ).await;
	let provider_addr2                     = provider_addr.clone();


	// Part that listens for incoming connection from the consumer to relay.
	// I havn't yet streamlined the API here like for the provider, so it's not
	// the service map that registers the services for you, so here we do a more
	// manual way of creating the peer actor and it's mailbox. It allows us to call
	// methods on the peer before starting it's mailbox which will consume the variable.
	//
	let relay = async move
	{
		let (srv_sink, srv_stream) = listen_tcp_stream( listen ).await;

		// Create mailbox for peer
		//
		let mb_peer  : Inbox<MyPeer> = Inbox::new()                  ;
		let peer_addr                = Addr ::new( mb_peer.sender() );

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::new( peer_addr, srv_stream.compat(), srv_sink.sink_compat() ).expect( "spawn peer" );

		let add  = <Add   as Service<remotes::Services>>::sid();
		let show = <Show  as Service<remotes::Services>>::sid();


		// We tell the peer what services to relay and what the peer address is of the peer that connects
		// to the provider. We also pass the event stream from the provider, so the peer can detect if the
		// provider goes offline and stop relaying these services.
		//
		// To deal with service interuptions, you can also register relayed services with a message type
		// `RegisterRelay` after the mailbox has consumed the peer var, but this is the prefered way, since
		// you peer actor will be completely set up before it starts processing incoming messages.
		//
		peer.register_relayed_services( vec![ add, show ], provider_addr2, provider_evts ).expect( "register relayed" );

		// start the mailbox
		//
		mb_peer.start_fut( peer ).await;
	};


	let (relay_fut, relay_outcome) = relay.remote_handle();
	rt::spawn( relay_fut ).expect( "failed to spawn server" );

	// we need to spawn this after peerb, otherwise peerb is not listening yet when we try to connect.
	//
	rt::spawn( next ).expect( "Spawn next"  );


	// If the nodec closes the connection, close our connection to provider.
	//
	relay_outcome.await;

	// Note that this will let the application terminate.
	//
	provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to provider" );
}




fn main()
{
	// flexi_logger::Logger::with_str( "remotes=trace, thespis_impl=trace, tokio=warn" ).start().unwrap();


	// Very similar to the basic example
	//
	let provider = async
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
		let _ = listen_tcp( "127.0.0.1:20000", sm ).await;

		trace!( "End of provider" );
	};


	// Very similar to the basic example
	//
	let consumer = async
	{
		let (mut relay, _)  = connect_to_tcp( "127.0.0.1:30000" ).await;

		// Call the service and receive the response
		//
		let mut add  = remotes::Services::recipient::<Add >( relay.clone() );
		let mut show = remotes::Services::recipient::<Show>( relay.clone() );

		let resp = add.call( Add(5) ).await.expect( "Call failed" );
		assert_eq!( (), resp );

		add.send( Add(5) ).await.expect( "Send failed" );

		let resp = show.call( Show ).await.expect( "Call failed" );
		println!( "If this was succesful you should see “10”: {:?}", &resp );

		assert_eq!( 10, resp );

		// Note that this will let the application terminate.
		//
		relay.send( CloseConnection{ remote: false } ).await.expect( "close connection to nodeb" );
	};

	let relays = async
	{
		relay( "127.0.0.1:20000", "127.0.0.1:30000", Box::pin( consumer ) ).await;
	};

	rt::spawn( provider  ).expect( "Spawn provider" );
	rt::spawn( relays    ).expect( "Spawn relays"   );

	rt::run();
}
