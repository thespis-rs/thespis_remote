//! # Thespis reference implementation
//!
//! ## Cargo Features
//!
//! - tokio: makes the tokio executor available. enabled by default.

#![ doc    ( html_root_url = "https://docs.rs/thespis_remote_impl" ) ]
#![ deny   ( /*missing_docs,*/ bare_trait_objects                      ) ]
#![ forbid ( unsafe_code                                           ) ]
#![ allow  ( clippy::suspicious_else_formatting                    ) ]

#![ warn
(
	missing_debug_implementations ,
	nonstandard_style             ,
	rust_2018_idioms              ,
	trivial_casts                 ,
	trivial_numeric_casts         ,
	unused_extern_crates          ,
	unused_qualifications         ,
	single_use_lifetimes          ,
	unreachable_pub               ,
	variant_size_differences      ,
)]


    mod error             ;
pub mod peer              ;
    mod relay_map         ;
    mod service_map_macro ;
    mod service_map       ;
    mod service_handler   ;
pub mod wire_format       ;

pub use
{
	error             :: * ,
	peer              :: * ,
	relay_map         :: * ,
	service_map       :: * ,
	service_map_macro :: * ,
	service_handler   :: * ,
	wire_format       :: * ,
};


// needed for macro
//
#[ doc( hidden ) ]
//
pub mod external_deps
{
	pub use futures      ;
	pub use log          ;
	pub use once_cell    ;
	pub use serde_cbor   ;
	pub use serde        ;
	pub use thespis      ;
	pub use thespis_impl ;
	pub use paste        ;
	pub use parking_lot  ;
}



// Import module. Avoid * imports here. These are all the foreign names that exist throughout
// the crate. They must all be unique.
// Separate use imports per enabled features.
//
mod import
{
	pub(crate) use
	{
		thespis        :: { *                                           } ,
		thespis_impl   :: { Addr, ThesErr                               } ,
		log            :: { *                                           } ,
		bytes          :: { Bytes, BytesMut, BufMut                     } ,
		rand           :: { Rng                                         } ,
		twox_hash      :: { XxHash64                                    } ,
		pharos         :: { Pharos, Observable, ObserveConfig, Events   } ,
		serde          :: { Serialize, Deserialize                      } ,
		thiserror      :: { Error                                       } ,
		once_cell      :: { sync::Lazy as SyncLazy                      } ,
		futures_timer  :: { Delay                                       } ,
		std ::
		{
			fmt                            ,
			convert     :: { TryFrom     } ,
			collections :: { HashMap     } ,
			sync        :: { Arc, Mutex  } ,
			hash        :: { Hasher      } ,
			time        :: { Duration    } ,
		},


		futures ::
		{
			prelude :: { Stream, Sink            } ,
			channel :: { oneshot                 } ,
			future  :: { FutureExt, RemoteHandle } ,
			sink    :: { SinkExt                 } ,
			stream  :: { StreamExt               } ,
			task    :: { Spawn, SpawnExt         } ,
		},
	};


	#[ cfg( feature = "tokio_codec" ) ]
	//
	pub(crate) use
	{
		tokio      :: { prelude::{ AsyncRead as TokioAsyncR, AsyncWrite as TokioAsyncW }                                      } ,
		tokio_util :: { codec::{ Decoder as TokioDecoder, Encoder as TokioEncoder, Framed as TokioFramed /*, FramedParts, FramedRead, FramedWrite*/ } } ,
	};


	#[ cfg( feature = "futures_codec" ) ]
	//
	pub(crate) use
	{
		futures_codec_crate ::
		{
			Encoder as FutEncoder,
			Decoder as FutDecoder,
			Framed  as FutFramed ,
		} ,

		futures::
		{
			AsyncRead  as FutAsyncRead  ,
			AsyncWrite as FutAsyncWrite ,
		}
	};

	#[ cfg(any( feature = "futures_codec", feature = "tokio_codec" )) ]
	//
	pub(crate) use
	{
		bytes:: { Buf } ,
	};



	#[ cfg(test) ]
	//
	pub(crate) use
	{
		pretty_assertions :: { assert_eq } ,
	};
}
