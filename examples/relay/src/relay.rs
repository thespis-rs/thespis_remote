mod common;

use common::{ import::*, * };


fn main()
{
	flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let exec = ThreadPool::new().expect( "create threadpool" );
	let     ex2  = exec.clone();

	let relays = async move
	{
		let producer_conn = TcpStream::connect( SERVER ).await.expect( "Connect to relay" );

		let     listener = TcpListener::bind( RELAY ).await.expect( "producer listen" );
		let mut incoming = listener.incoming();

		let consumer_conn = incoming.next().await.expect( "one connection" ).expect( "valid tcp" );


		relay( producer_conn, consumer_conn, true, ex2 ).await;

		warn!( "relays end" );
	};

	block_on( relays );
}



// Helper method to create relays
//
async fn relay
(
	connect   : TcpStream,
	listen    : TcpStream,
	relay_show: bool,
	exec      : impl SpawnHandle + Send + Sync + Clone + 'static
)
{
	debug!( "start mailbox for relay_to_provider" );

	let (mut provider_addr, _provider_evts) = peer_connect( connect, exec.clone(), "relay_to_provider" ).await;
	let provider_addr2                     = provider_addr.clone();

	debug!( "Actor for relay_to_provider is {}", provider_addr2.id() );

	// Relay part ---------------------

	let ex2 = exec.clone();

	let relay = async move
	{
		let codec: ThesCodec = ThesCodec::new(1024);

		let (client_sink, client_stream) = Framed::new( listen, codec ).split();

		// Create mailbox for the client peer
		//
		let mb_peer  : Inbox<Peer> = Inbox::new( Some( "relay_to_consumer".into() ) );
		let peer_addr              = Addr ::new( mb_peer.sender() );

		// This peer is listening for the connection from the client.
		//
		let mut peer = Peer::new( peer_addr, client_stream, client_sink, ex2 ).expect( "spawn peer" );

		let add  = <Add  as remotes::Service>::sid();
		let show = <Show as remotes::Service>::sid();

		let rm      = Arc::new( RelayMap::new() );
		let closure = Box::new( move |_: &ServiceID| Some( Box::new(provider_addr2.clone()) as Box<dyn Relay> ) );

		rm.register_handler( add.clone(), closure.clone() );

		if relay_show
		{
			rm.register_handler( show.clone(), closure );
		}


		peer.register_services( rm );

		debug!( "start mailbox for relay_to_consumer" );
		mb_peer.start_fut( peer ).await;
		warn!( "relay async block finished" );
	};


	// If the nodec closes the connection, close our connection to provider.
	//
	exec.spawn_handle( relay ).expect( "failed to spawn server" ).await;
	warn!( "relay finished, closing connection" );

	provider_addr.send( CloseConnection{ remote: false } ).await.expect( "close connection to provider" );
}
