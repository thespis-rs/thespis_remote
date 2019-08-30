// The macro uses box syntax for now, so we enable it.
//
#![ feature( box_syntax ) ]


// In common we have the definitions of some simple Actors and a few helper functions to setup
// the network connection.
//
// If you want to understand this example, it's a good idea to read common/common.rs and common/actors.rs.
//
mod common;

use common::*                       ;
use common::import::{ *, assert_eq };


fn main()
{
	// This might be separate processes, but here we just use async tasks.
	//
	let peera = async
	{
		// Create mailbox for our handler. The struct Sum will handle the messages for the Services Add and Show.
		// A Service is like a Message, but it has to support Serialize, Deserialize and have a unique identifier.
		//
		let addr_handler = Addr::try_from( Sum(0) ).expect( "spawn actor mailbox" );

		// Create a service map.
		// A service map is a helper object created by a beefy macro included with thespis_impl_remote. It is responsible
		// for deserializing and delivering the message to the correct handler.
		//
		let mut sm = remotes::Services::new();

		// Register our handlers.
		// We say that Sum from above will provide these services.
		//
		sm.register_handler::<Add >( Receiver::new( addr_handler.recipient() ) );
		sm.register_handler::<Show>( Receiver::new( addr_handler.recipient() ) );

		// get a framed connection.
		// This method does quite a lot, you might want to look it up in common/common.rs.
		//
		let _ = listen_tcp( "127.0.0.1:8998", sm ).await;
	};


	let peerb = async
	{
		// get a framed connection.
		// This method does quite a lot, you might want to look it up in common/common.rs.
		//
		let (mut peera, _) = connect_to_tcp( "127.0.0.1:8998" ).await;

		// The service map also provides a utility to create recipients for remote actors. We have
		// to tell it over which connection to send messages by passing the address of a peer.
		// There is no verification that the remote process actually provides these services. You
		// have to guarantee that.
		//
		// the `remotes::Services` type comes from the `service_map!` macro being called in common.rs.
		//
		let mut add  = remotes::Services::recipient::<Add >( peera.clone() );
		let mut show = remotes::Services::recipient::<Show>( peera.clone() );

		// Call the service and receive the response. Just as if it were a local actor handling
		// this.
		//
		let resp = add.call( Add(5) ).await.expect( "Call failed" );
		assert_eq!( (), resp );

		// Both call and send work
		//
		add.send( Add(5) ).await.expect( "Send failed" );

		let resp = show.call( Show ).await.expect( "Call failed" );
		println!( "If this was succesful you should see “10”: {:?}", &resp );

		assert_eq!( 10, resp );

		// Note that this will let the application terminate. A peer needs to have it's own address for
		// implementation reasons (it needs to send itself messages). This means that it keeps itself from
		// being dropped.
		//
		// When we send CloseConnection, or when the remote closes the connection, the peer will try to
		// drop any data it holds, including it's own address, so it can be dropped. If you however
		// still hold it's address, you will keep it alive. To avoid this problem, "observe" the actor
		// and drop all addresses to it when you see a connection closed event.
		//
		// There will be a separate example illustrating how to observe events from peers.
		//
		peera.send( CloseConnection{ remote: false } ).await.expect( "close connection to peera" );
	};


	rt::spawn( peera  ).expect( "Spawn peera"  );
	rt::spawn( peerb  ).expect( "Spawn peerb"  );

	rt::run();
}
