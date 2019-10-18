pub mod import
{
	pub use
	{
		thespis             :: { *                      } ,
		thespis_remote      :: { *                      } ,
		thespis_remote_impl :: { *, service_map, peer   } ,
		futures_codec       :: { Framed                 } ,
		bytes               :: { Bytes, BytesMut        } ,
		serde               :: { Serialize, Deserialize } ,

		std::
		{
			fmt,
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


pub type MS = MultiServiceImpl<ServiceID, ConnID, Codecs>;


#[ derive( Serialize, Deserialize, Debug, Clone, Copy ) ]
//
pub struct Ping(pub usize);

impl Message for Ping { type Return = usize; }




service_map!
(
	namespace:     remotes ;
	multi_service: MS      ;
	services     : Ping    ;
);



