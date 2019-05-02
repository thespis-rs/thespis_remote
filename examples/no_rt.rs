#![ feature( await_macro, async_await, arbitrary_self_types, specialization, nll, never_type, unboxed_closures, trait_alias ) ]

#![ allow( dead_code, unused_imports )]


use
{
	futures       :: { future::{ Future, FutureExt }, task::{ LocalSpawn, SpawnExt, LocalSpawnExt }, executor::LocalPool  } ,
	std           :: { pin::Pin } ,
	log           :: { *        } ,
	thespis       :: { *        } ,
	thespis_impl  :: { *        } ,
};


#[ derive( Actor ) ]
//
struct MyActor;

struct Ping( String );


impl Message for Ping
{
	type Return = String;
}


impl Handler< Ping > for MyActor
{
	fn handle( &mut self, _msg: Ping ) -> Return<String> { Box::pin( async move
	{
		"pong".into()

	})}
}



fn main()
{
	let mut pool  = LocalPool::new();
	let mut exec  = pool.spawner();
	let mut exec2 = exec.clone();

	let program = async move
	{
		let a = MyActor;

		// Create mailbox
		//
		let mb  : Inbox<MyActor> = Inbox::new();
		let mut addr  = Addr::new( mb.sender() );

		// TODO: This is ugly right now. It will be more ergonomic in the future.
		//
		let move_mb = async move { await!( mb.start_fut( a ) ); };
		exec2.spawn_local( move_mb ).expect( "Spawning mailbox failed" );

		let result  = await!( addr.call( Ping( "ping".into() ) ) ).expect( "Call failed" );

		assert_eq!( "pong".to_string(), result );
		dbg!( result );
	};

	exec.spawn_local( program ).expect( "Spawn program" );

	pool.run();
}
