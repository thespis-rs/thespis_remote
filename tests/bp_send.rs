// Tests:
// 
// ✔ sends do not count to backpressure counter (semaphore)
// ✔ sends still work when we are blocked on backpressure.
// 
mod common;

use
{
	common            :: { *, import::*              } ,
	serde             :: { Serialize, Deserialize    } ,
	tokio::sync       :: { Semaphore                 } ,
	pretty_assertions :: { assert_eq                 } ,
	async_progress    :: { Progress                  } ,
	futures::channel  :: { mpsc::{ UnboundedSender } } ,
};


type BoxFut = Pin<Box< dyn Future<Output=()> + Send >>;


#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash) ]
//
enum Steps
{
	Start,
	BlockOne,
	BlockTwo,
	End,
}




#[ derive(Actor) ] struct Slow
{
	p   : Progress<Steps>               ,
	b1  : BoxFut                        ,
	end : BoxFut                        ,
	end2: BoxFut                        ,
	tx  : UnboundedSender<&'static str> ,
}


impl Slow
{
	pub async fn new( p: Progress<Steps>, tx: UnboundedSender<&'static str> ) -> Self
	{
		let b1   = p.once( Steps::BlockOne ).map(|_|()).boxed();
		let end  = p.once( Steps::End      ).map(|_|()).boxed();
		let end2 = p.once( Steps::End      ).map(|_|()).boxed();

		Slow { p, b1, end, end2, tx }
	}
}



#[ derive( Serialize, Deserialize, Debug ) ] pub struct A;
#[ derive( Serialize, Deserialize, Debug ) ] pub struct B;
#[ derive( Serialize, Deserialize, Debug ) ] pub struct C;

impl Message for A { type Return = ();  }
impl Message for B { type Return = ();  }
impl Message for C { type Return = ();  }



impl Handler<A> for Slow
{
	#[async_fn] fn handle( &mut self, _msg: A )
	{
		warn!( "handle A" );

		self.p.set_state( Steps::BlockOne ).await;

		Pin::new( &mut self.end ).await;
		self.tx .send( "A" ).await.expect( "send to unbounded" );
	}
}



impl Handler<B> for Slow
{
	#[async_fn] fn handle( &mut self, _msg: B )
	{
		warn!( "handle B" );

		Pin::new( &mut self.b1 ).await;

		self.p.set_state( Steps::BlockTwo ).await;

		Pin::new( &mut self.end2 ).await;
		self.tx .send( "B" ).await.expect( "send to unbounded" );
	}
}



impl Handler<C> for Slow
{
	#[async_fn] fn handle( &mut self, _msg: C )
	{
		warn!( "handle C" );
		self.tx .send( "C" ).await.expect( "send to unbounded" );

		self.p.set_state( Steps::End ).await;
	}
}



service_map!
(
	namespace  : bpsm    ;
	wire_format: CborWF  ;
	services   : A, B, C ;
);






// We create a peer with backpressure 2. Then send in an call and a send that will block.
// Since the send should not take up a slot, we can now do another call.
//
// We verify the order in which tasks ran by looking at the order they signal a channel.
//
#[async_std::test]
//
async fn bp_send_does_not_consume_slot()
{
	let progress         = Progress::new( Steps::Start );
	let progress2        = progress.clone();
	let (tx, mut rx)     = futures::channel::mpsc::unbounded();
	let tx2              = tx.clone();
	let tx3              = tx.clone();
	let (server, client) = Endpoint::pair( 64, 64 );

	let peera = async move
	{
		// Create mailbox for our handler
		//
		let slow  = Addr::builder().spawn( Slow::new( progress.clone(), tx  ).await, &AsyncStd ).expect( "spawn actor mailbox" );
		let slow2 = Addr::builder().spawn( Slow::new( progress.clone(), tx2 ).await, &AsyncStd ).expect( "spawn actor mailbox" );
		let slow3 = Addr::builder().spawn( Slow::new( progress.clone(), tx3 ).await, &AsyncStd ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let mut sm = bpsm::Services::new();

		// Register our handlers
		//
		sm.register_handler::<A>( slow .clone_box() );
		sm.register_handler::<B>( slow2.clone_box() );
		sm.register_handler::<C>( slow3.clone_box() );

		// create peer with stream/sink
		//
		let (mut peer, peer_mb, _peer_addr) = CborWF::create_peer
		(
			"server", server, 1024, 1024,
			AsyncStd,
			Some(Arc::new( Semaphore::new(2) )),
			None

		).expect( "spawn peer" );


		// register service map with peer
		//
		peer.register_services( Arc::new( sm ) );

		let handle = AsyncStd.spawn_handle_local( peer_mb.start(peer) ).expect( "start mailbox of Peer" );
		handle.await;

		trace!( "end of peera" );
	};


	let peerb = async move
	{
		let (mut peera, _)  = peer_connect( client, AsyncStd, "peer_b_to_peera" ).await;

		// Call the service and receive the response
		//
		let mut addr     = bpsm::RemoteAddr::new( peera.clone() );
		let mut addr2    = addr.clone();
		let mut addr3    = addr.clone();


		let a = async move { addr .call( A ).await.expect( "call add1"  ); };
		let b = async move { addr2.send( B ).await.expect( "call add2"  ); };
		let c = async move { addr3.call( C ).await.expect( "call check" ); };

		let a_handle = AsyncStd.spawn_handle( a ).expect( "spawn add1"  );
		let b_handle = AsyncStd.spawn_handle( b ).expect( "spawn add2"  );

		progress2.once( Steps::BlockTwo ).await;

		let c_handle = AsyncStd.spawn_handle( c ).expect( "spawn check" );


		a_handle.await;
		b_handle.await;
		c_handle.await;

		assert_eq!( rx.next().await.unwrap(), "C" );

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	join( peera, peerb ).await;
}




// We create a peer with backpressure 2. Then send in an 2 calls that will block.
// Since the send should not take up a slot, we can send it in, even if the 2 calls are still blocking.
//
// We verify the order in which tasks ran by looking at the order they signal a channel.
//
#[async_std::test]
//
async fn bp_send_gets_in_when_backed_up()
{
	let progress         = Progress::new( Steps::Start );
	let progress2        = progress.clone();
	let (tx, mut rx)     = futures::channel::mpsc::unbounded();
	let tx2              = tx.clone();
	let tx3              = tx.clone();
	let (server, client) = Endpoint::pair( 64, 64 );

	let peera = async move
	{
		// Create mailbox for our handler
		//
		let slow  = Addr::builder().spawn_local( Slow::new( progress.clone(), tx  ).await, &AsyncStd ).expect( "spawn actor mailbox" );
		let slow2 = Addr::builder().spawn_local( Slow::new( progress.clone(), tx2 ).await, &AsyncStd ).expect( "spawn actor mailbox" );
		let slow3 = Addr::builder().spawn_local( Slow::new( progress.clone(), tx3 ).await, &AsyncStd ).expect( "spawn actor mailbox" );

		// Create a service map
		//
		let mut sm = bpsm::Services::new();

		// Register our handlers
		//
		sm.register_handler::<A>( slow .clone_box() );
		sm.register_handler::<B>( slow2.clone_box() );
		sm.register_handler::<C>( slow3.clone_box() );

		// create peer with stream/sink
		//
		let (mut peer, peer_mb, _peer_addr) = CborWF::create_peer
		(
			"server", server, 1024, 1024,
			AsyncStd,
			Some(Arc::new( Semaphore::new(2) )),
			None

		).expect( "spawn peer" );


		// register service map with peer
		//
		peer.register_services( Arc::new( sm ) );

		let handle = AsyncStd.spawn_handle_local( peer_mb.start(peer) ).expect( "start mailbox of Peer" );
		handle.await;

		trace!( "end of peera" );
	};


	let peerb = async move
	{
		let (mut peera, _)  = peer_connect( client, AsyncStd, "peer_b_to_peera" ).await;

		// Call the service and receive the response
		//
		let mut addr     = bpsm::RemoteAddr::new( peera.clone() );
		let mut addr2    = addr.clone();
		let mut addr3    = addr.clone();


		let a = async move { addr .call( A ).await.expect( "call add1"  ); };
		let b = async move { addr2.call( B ).await.expect( "call add2"  ); };
		let c = async move { addr3.send( C ).await.expect( "call check" ); };

		let a_handle = AsyncStd.spawn_handle_local( a ).expect( "spawn add1"  );
		let b_handle = AsyncStd.spawn_handle_local( b ).expect( "spawn add2"  );

		progress2.once( Steps::BlockTwo ).await;

		let c_handle = AsyncStd.spawn_handle_local( c ).expect( "spawn check" );


		a_handle.await;
		b_handle.await;
		c_handle.await;

		assert_eq!( rx.next().await.unwrap(), "C" );

		peera.send( CloseConnection{ remote: false, reason: "Program end.".to_string() } ).await.expect( "close connection to peera" );
	};


	// As far as I can tell, execution order is not defined, so hmm, there is no
	// guarantee that a is listening before b tries to connect, but it seems to work for now.
	//
	join( peera, peerb ).await;
}
