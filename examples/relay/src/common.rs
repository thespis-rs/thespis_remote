#![ allow( dead_code ) ]



pub mod import
{
	pub use
	{
		async_std           :: { net::{ TcpListener, TcpStream }, io::{ Read as _, Write as _ } } ,
		async_executors     :: { AsyncStd, SpawnHandle, SpawnHandleExt, LocalSpawnHandle        } ,
		futures_ringbuf     :: { Endpoint                                                       } ,
		thespis             :: { *                                                              } ,
		thespis_impl        :: { *                                                              } ,
		thespis_remote      :: { *, service_map, peer                                           } ,
		log                 :: { *                                                              } ,
		bytes               :: { Bytes, BytesMut                                                } ,
		pharos              :: { Observable, ObserveConfig, Events                              } ,
		serde               :: { Serialize, Deserialize                                         } ,

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
			executor:: { block_on, ThreadPool                                                    } ,
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


pub fn peer_connect
(
	socket: TcpStream                                   ,
	exec  : impl Spawn + SpawnHandle< Result<Response, PeerErr> > + Clone + Send + Sync + 'static ,
	name  : &str                                       ,
)
	-> (Addr<Peer>, Events<PeerEvent>)
{
	// Create mailbox for peer
	//
	let (peer_addr, peer_mb) = Addr::builder().name( name.into() ).build();


	// create peer with stream/sink + service map
	//
	let mut peer = Peer::from_async_read( peer_addr.clone(), socket, 1024, exec.clone(), None ).expect( "spawn peer" );

	let evts = peer.observe( ObserveConfig::default() ).expect( "pharos not closed" );

	debug!( "start mailbox for [{}] in peer_connect", name );

	exec.spawn( async{ peer_mb.start(peer).await; } ).expect( "start mailbox of Peer" );

	(peer_addr, evts)
}


service_map!
(
	namespace  :     remotes ;
	wire_format: ThesWF      ;
	services   : Add, Show   ;
);

