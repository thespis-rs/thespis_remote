mod common;

use common::{ import::*, * };


#[ async_std::main ]
//
async fn main()
{
	flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let producer_conn = TcpStream::connect( SERVER ).await.expect( "Connect to relay" );
	let     listener  = TcpListener::bind( RELAY ).await.expect( "producer listen" );
	let mut incoming  = listener.incoming();

	let consumer_conn = incoming.next().await.expect( "one connection" ).expect( "valid tcp" );

	relay( producer_conn, consumer_conn, true, AsyncStd ).await;

	warn!( "relays end" );
}



// Helper method to create relays
//
async fn relay
(
	connect   : TcpStream,
	listen    : TcpStream,
	relay_show: bool,
	exec      : impl Spawn + SpawnHandle<()> + SpawnHandle< Result<Response, PeerErr> > + Clone + Send + Sync + 'static,
)
{
	debug!( "start mailbox for relay_to_provider" );

	let (mut provider_addr, _provider_evts) = peer_connect( connect, exec.clone(), "relay_to_provider" );
	let provider_addr2                      = provider_addr.clone();

	debug!( "Actor for relay_to_provider is {}", provider_addr2.id() );

	// Relay part ---------------------

	let ex2 = exec.clone();

	let relay = async move
	{
		// Create mailbox for peer
		//
		let (peer_addr, peer_mb) = Addr::builder().name( "relay_to_consumer".into() ).build();

		// create peer with stream/sink + service map
		//
		let mut peer = Peer::from_async_read( peer_addr, listen, 1024, ex2, None ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();

		let mut services = vec![ add.clone() ];

		if relay_show
		{
			services.push( show.clone() );
		}

		let rm = Arc::new( RelayMap::new( ServiceHandler::Address( Box::new( provider_addr2.clone() ) ), services ) );

		peer.register_services( rm );

		debug!( "start mailbox for relay_to_consumer" );
		peer_mb.start( peer ).await;
		warn!( "relay async block finished" );
	};


	// If the nodec closes the connection, close our connection to provider.
	//
	exec.spawn_handle( relay ).expect( "failed to spawn server" ).await;
	warn!( "relay finished, closing connection" );

	provider_addr.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to provider" );
}
