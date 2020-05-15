// Tests:
//
// ✔ Provoke an internal_server_error and make sure we get it back.
// - all edge cases, all different errors, error on the very last request before a connection get's closed,
//   does the nursery get properly cleaned up when peer is dropped, ...
//
mod common;

use common::*                       ;
use common::remotes::Service        ;
use common::import::{ * };


// Test basic remote funcionality. Test intertwined sends and calls.
//
#[async_std::test]
//
async fn no_handler()
{
	// flexi_logger::Logger::with_str( "trace" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let peera = async move
	{
		// Create mailbox for our handler
		//
		let (addr_handler, _) = Addr::<Sum>::builder().build();

		// Create a service map
		//
		let mut sm = remotes::Services::new();

		// Register our handlers
		//
		sm.register_handler::<Add >( addr_handler.clone_box() );
		sm.register_handler::<Show>( addr_handler.clone_box() );

		// get a framed connection
		//
		let (_, _, handle) = peer_listen( server, Arc::new( sm ), AsyncStd, "peera" );

		handle.await;

		trace!( "end of peera" );
	};


	let peerb = async move
	{
		let (mut peera, _)  = peer_connect( client, AsyncStd, "peer_b_to_peera" );

		// Call the service and receive the response
		//
		let mut addr = remotes::RemoteAddr::new( peera.clone() );

		let res = addr.call( Add(5) ).await;

		assert!(matches!( res,

			Err
			(
				PeerErr::Remote
				{
					ctx: _,
					err: ConnectionError::InternalServerError
					{
						sid,
						cid: _,
					}
				}
			)

				if sid == Some(Add::sid())
		));


		let res = addr.call( Show ).await;

		assert!(matches!( res,

			Err
			(
				PeerErr::Remote
				{
					ctx: _,
					err: ConnectionError::InternalServerError
					{
						sid,
						cid: _,
					}
				}
			)

				if sid == Some(Show::sid())
		));

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	join( peera, peerb ).await;
}


