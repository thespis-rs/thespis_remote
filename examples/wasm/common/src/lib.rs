pub mod import
{
	pub use
	{
		thespis             :: { *                      } ,
		thespis_remote      :: { *, service_map, peer   } ,
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
			stream  :: { StreamExt, SplitSink, SplitStream                                       } ,
			future  :: { FutureExt                                                               } ,
		},
	};
}


use import::*;


#[ derive( Serialize, Deserialize, Debug, Clone, Copy ) ]
//
pub struct Ping(pub usize);

impl Message for Ping { type Return = usize; }




service_map!
(
	namespace:     remotes ;
	services     : Ping    ;
);



