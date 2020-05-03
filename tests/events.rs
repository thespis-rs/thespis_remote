#![ cfg( feature = "futures_codec" ) ]

// This currently combines testing error handling and peer events, because often the events are the way
// to be notified of errors.
//
// Tests:
//
// - ✔ Connection Closed events (closed by remote tested in basic_use)
// - ✔ Connection Closed events (send+call)
// - ✔ Header Deserialization (Remote)Error (Codec)
// - ✔ Invalid Codec
// - ✔ Header Unknown Service (Remote)Error
// - ✔ Service map Deserialization (Remote)Error
// - ✔ handling remote errors on call (let the caller know there were connection errors) -> tested in relay.rs
//
// - TODO: fuzz, SEND A WHOLE BUNCH OF BINARY DATA OVER THE NETWORK AND VERIFY THE CORRECT ERROR FOR EACH TYPE OF INPUT.


mod common;

use common::*                       ;
use common::import::{ *, assert_eq };



// Test calling a remote service after the connection has closed.
//
#[async_std::test]
//
async fn close_connection()
{
	let (server, client) = Endpoint::pair( 64, 64 );

	let nodea = async move
	{
		// get a framed connection
		//
		let (_, _, handle) = peer_listen( server, Arc::new( add_show_sum() ), AsyncStd, "nodea" );

		handle.await;
	};


	let nodeb = async move
	{
		let (mut peera, mut peera_evts)  = peer_connect( client, AsyncStd, "nodeb_to_nodea" );

		// Close the connection and check the event
		//
		peera.send( peer::CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "Send CloseConnection" );

		assert_eq!( PeerEvent::Closed,  peera_evts.next().await.unwrap() );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	let a_handle = AsyncStd.spawn_handle( nodea ).expect( "Spawn peera"  );
	let b_handle = AsyncStd.spawn_handle( nodeb ).expect( "Spawn peerb"  );

	join( a_handle, b_handle ).await;
}



// Test calling a remote service after the connection has closed.
//
#[async_std::test]
//
async fn close_connection_call()
{
	let (server, client) = Endpoint::pair( 64, 64 );

	let nodea = async move
	{
		// get a framed connection
		//
		let (_, _, handle) = peer_listen( server, Arc::new( add_show_sum() ), AsyncStd, "nodea" );

		handle.await;
	};


	let nodeb = async move
	{
		let (mut peera, mut peera_evts)  = peer_connect( client, AsyncStd, "nodeb_to_nodea" );

		// Close the connection and check the event
		//
		peera.call( peer::CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "Send CloseConnection" );

		assert_eq!( PeerEvent::Closed, peera_evts.next().await.unwrap() );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	let a_handle = AsyncStd.spawn_handle( nodea ).expect( "Spawn peera"  );
	let b_handle = AsyncStd.spawn_handle( nodeb ).expect( "Spawn peerb"  );

	join( a_handle, b_handle ).await;
}



// Test Header Unknown Service (Remote)Error.
//
#[async_std::test]
//
async fn header_unknown_service_error()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=trace, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );


	let nodea = async move
	{
		// get a framed connection
		//
		let (_, mut evts, handle) = peer_listen( server, Arc::new( add_show_sum() ), AsyncStd, "nodea" );

		let sid = Some( ServiceID::from( Bytes::from( vec![3;16] ) ) );

		match evts.next().await.unwrap()
		{
			PeerEvent::Error( PeerErr::UnknownService{ ctx } ) =>
			{
				assert_eq!( ctx.sid, sid );
			}

			_ => unreachable!(),
		}

		handle.await;
	};


	let nodeb = async move
	{
		let (mut peera, mut peera_evts)  = peer_connect( client, AsyncStd, "nodeb_to_nodea" );

		// Create some random data that shouldn't deserialize
		//
		let sid = ServiceID::from( Bytes::from( vec![3;16] ) );
		let cid = ConnID::random();
		let ms  = WireFormat::create( sid.clone(), cid.clone(), serde_cbor::to_vec( &Add(5) ).expect( "serialize Add(5)" ).into() );

		peera.call( ms ).await.expect( "call peera" ).expect( "call peera" );

		assert_eq!
		(
			PeerEvent::RemoteError( ConnectionError::UnknownService{ sid: Some( sid ), cid: cid.into() } ),
			peera_evts.next().await.unwrap()
		);

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	let a_handle = AsyncStd.spawn_handle( nodea ).expect( "Spawn peera"  );
	let b_handle = AsyncStd.spawn_handle( nodeb ).expect( "Spawn peerb"  );

	join( a_handle, b_handle ).await;
}




// Test Call deserialize (Remote) Error.
//
#[async_std::test]
//
async fn call_deserialize()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=debug, thespis_remote=trace, tokio=warn" ).start().unwrap();
	//
	let (server, client) = Endpoint::pair( 64, 64 );

	let nodea = async move
	{
		// get a framed connection
		//
		let (_, mut evts, handle) = peer_listen( server, Arc::new( add_show_sum() ), AsyncStd, "nodea" );


		match evts.next().await.unwrap()
		{
			PeerEvent::Error( PeerErr::Deserialize{ ctx } ) =>
			{
				assert_eq!( ctx.context.unwrap(), "Services::call_service" );
			}

			_ => unreachable!( "Should be PeerEvent::Error( PeerErr::Deserialize" )
		}

		handle.await;
	};


	let nodeb = async move
	{
		let (mut peera, mut peera_evts)  = peer_connect( client, AsyncStd, "nodeb_to_nodea" );

		// Create some random data that shouldn't deserialize
		//
		let sid: Bytes = <Add as remotes::Service>::sid().clone().into();
		let cid: Bytes = ConnID::random().into();
		let msg: Bytes = serde_cbor::to_vec( &Add(5) ).unwrap().into();

		// This is the corrupt one that should trigger a deserialization error and close the connection
		//
		let cod = Bytes::from( vec![ 1,2,3,4 ] );

		let mut buf = BytesMut::new();

		buf.extend( sid         );
		buf.extend( cid.clone() );
		buf.extend( cod         );
		buf.extend( msg         );

		let mesg  = WireFormat::try_from( buf.freeze() ).expect( "serialize Add(5)" );

		peera.call( mesg ).await.expect( "send ms to peera" ).expect( "no network error" );

		assert_eq!
		(
			PeerEvent::RemoteError(ConnectionError::Deserialize
			{
				sid: <Add as remotes::Service>::sid().clone().into() ,
				cid: ConnID::from( cid ).into()                      ,
			}),

			peera_evts.next().await.unwrap()
		);

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	let a_handle = AsyncStd.spawn_handle( nodea ).expect( "Spawn peera"  );
	let b_handle = AsyncStd.spawn_handle( nodeb ).expect( "Spawn peerb"  );

	join( a_handle, b_handle ).await;
}



// Test Service map Deserialization (Remote)Error.
//
#[async_std::test]
//
async fn sm_deserialize_error()
{
	// flexi_logger::Logger::with_str( "events=trace, thespis_impl=trace, thespis_remote_impl=trace, tokio=warn" ).start().unwrap();

	let (server, client) = Endpoint::pair( 64, 64 );

	let nodea = async move
	{
		// get a framed connection
		//
		let (_, mut evts, handle) = peer_listen( server, Arc::new( add_show_sum() ), AsyncStd, "nodea" );

		match evts.next().await.unwrap()
		{
			PeerEvent::Error( PeerErr::Deserialize{ ctx } ) =>
			{
				assert_eq!( ctx.context.unwrap(), "Services::call_service" );
			}

			_ => unreachable!( "Should be PeerEvent::Error( PeerErr::Deserialize" )
		}

		handle.await;
	};


	let nodeb = async move
	{
		let (mut peera, mut peera_evts) = peer_connect( client, AsyncStd, "nodeb_to_nodea" );

		// Create some random data that shouldn't deserialize
		//
		let sid = <Add as remotes::Service>::sid().clone();
		let cid = ConnID::random();
		let ms  = WireFormat::create( sid, cid.clone(), Bytes::from( vec![3,3]));

		peera.call( ms ).await.expect( "call peera" ).expect( "call peera" );

		assert_eq!
		(
			PeerEvent::RemoteError(ConnectionError::Deserialize
			{
				sid: <Add as remotes::Service>::sid().clone().into() ,
				cid: cid.into()                                      ,
			}),

			peera_evts.next().await.unwrap()
		);

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	let a_handle = AsyncStd.spawn_handle( nodea ).expect( "Spawn peera"  );
	let b_handle = AsyncStd.spawn_handle( nodeb ).expect( "Spawn peerb"  );

	join( a_handle, b_handle ).await;
}
