pub mod import
{
	pub use
	{
		thespis             :: { *                    } ,
		thespis_remote      :: { *                    } ,
		thespis_impl_remote :: { *, service_map, peer } ,
		futures_codec       :: { Framed                  } ,
		bytes               :: { Bytes, BytesMut      } ,
		serde               ::{ Serialize, Deserialize } ,

		std::
		{
			fmt,
			ops     :: Deref               ,
			net     :: SocketAddr          ,
			convert :: TryFrom             ,
			future  :: Future              ,
			pin     :: Pin                 ,
			error   :: Error as ErrorTrait ,
		},

		futures::
		{
			channel :: { mpsc                                                                    } ,
			compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
			stream  :: { StreamExt, SplitSink, SplitStream                                       } ,
			future  :: { FutureExt                                                               } ,
		},
	};
}


use import::*;

pub mod error;
pub use error::*;




/// Services exposed by the server
//
#[ derive( Serialize, Deserialize, Debug, Clone ) ] pub struct ChatMsg( pub String );
#[ derive( Serialize, Deserialize, Debug, Clone ) ] pub struct SetNick{ pub nick: String }
#[ derive( Serialize, Deserialize, Debug, Clone ) ] pub struct Join   { pub nick: String }

impl Message for Join    { type Return = Result<Welcome, ChatErr> ; }
impl Message for SetNick { type Return = Result<()   , ChatErr>   ; }
impl Message for ChatMsg { type Return = ()                       ; }


/// Services exposed by clients
//
#[ derive( Serialize, Deserialize, Debug, Clone ) ]
//
pub enum ServerMsg
{
	ChatMsg       { time: i64, sid: usize, txt: String   } ,
	UserJoined    { time: i64, nick : String, sid: usize } ,
	UserLeft      { time: i64, sid: usize                } ,
	NickChanged   { time: i64, sid: usize, nick: String  } ,
}

impl Message for ServerMsg { type Return = (); }


/// Response type for a successful join
//
#[ derive( Serialize, Deserialize, Debug, Clone ) ]
//
pub struct Welcome
{
	pub time : i64,
	pub sid  : usize,
	pub users: Vec<(usize,String)>,
	pub txt  : String
}



// service_map!
// (
// 	namespace    : client_map ;
// 	peer_type    : ClientConn ;
// 	multi_service: MS         ;
// 	services     : ServerMsg  ;
// );

