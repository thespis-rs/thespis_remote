#![ allow( dead_code ) ]



pub mod import
{
	pub use
	{
		async_runtime as rt,
		async_std           :: { net::{ TcpListener, TcpStream }, io::{ Read as _, Write as _ }             } ,
		async_executors     :: { LocalPool, AsyncStd, ThreadPool, JoinHandle, SpawnHandle, LocalSpawnHandle } ,
		futures_ringbuf     :: { Endpoint                                                                   } ,
		thespis             :: { *                                                                          } ,
		thespis_impl        :: { *                                                                          } ,
		thespis_remote_impl :: { *, service_map, peer                                                       } ,
		log                 :: { *                                                                          } ,
		bytes               :: { Bytes, BytesMut                                                            } ,
		pharos              :: { Observable, ObserveConfig, Events                                          } ,
		serde               :: { Serialize, Deserialize                                                     } ,

		std::
		{
			net     :: SocketAddr ,
			convert :: TryFrom    ,
			future  :: Future     ,
			pin     :: Pin        ,
			sync    :: Arc        ,
		},

		futures::
		{
			prelude :: { Sink, Stream                                                            } ,
			channel :: { mpsc                                                                    } ,
			io      :: { AsyncWriteExt, AsyncRead, AsyncWrite                                    } ,
			compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink, SplitStream                                       } ,
			future  :: { FutureExt, join                                                         } ,
			task    :: { SpawnExt, LocalSpawnExt, Spawn                                          } ,
			executor:: { block_on                                                                } ,
		},


		futures_codec :: { Decoder, Framed, FramedWrite } ,
	};
}

use import::*;

pub const SERVER: &str = "127.0.0.1:3012";
pub const RELAY : &str = "127.0.0.1:3013";



#[ derive( Actor ) ] pub struct Sum( pub u64 );

#[ derive( Serialize, Deserialize, Debug ) ] pub struct Add( pub u64 );
#[ derive( Serialize, Deserialize, Debug ) ] pub struct Show;

impl Message for Add  { type Return = ();  }
impl Message for Show { type Return = u64; }



impl Handler< Add > for Sum
{
	fn handle( &mut self, msg: Add ) -> Return<()> { Box::pin( async move
	{
		trace!( "called sum with: {:?}", msg );

		self.0 += msg.0;

	})}
}



impl Handler< Show > for Sum
{
	fn handle( &mut self, _msg: Show ) -> Return<u64> { Box::pin( async move
	{
		trace!( "called sum with: Show" );

		self.0

	})}
}



pub async fn peer_connect( socket: TcpStream, exec: &impl Spawn, name: &'static str ) -> (Addr<Peer>, Events<PeerEvent>)
{
	// Create mailbox for peer
	//
	let mb  : Inbox<Peer> = Inbox::new( name.into() );
	let addr              = Addr ::new( mb.sender() );

	// create peer with stream/sink + service map
	//
	let mut peer = Peer::from_async_read( addr.clone(), socket, 1024 ).expect( "create peer" );

	let evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( mb.start_fut(peer) ).expect( "start mailbox of Peer" );

	(addr, evts)
}


service_map!
(
	namespace:     remotes   ;
	services     : Add, Show ;
);

