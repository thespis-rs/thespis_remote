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
		thespis         :: { *                                           } ,
		thespis_impl    :: { Addr, ThesErr                               } ,
		log             :: { *                                           } ,
		bytes           :: { Bytes, BytesMut, BufMut                     } ,
		rand            :: { Rng                                         } ,
		twox_hash       :: { XxHash64                                    } ,
		pharos          :: { Pharos, Observable, ObserveConfig, Events   } ,
		serde           :: { Serialize, Deserialize                      } ,
		thiserror       :: { Error                                       } ,
		once_cell       :: { sync::Lazy as SyncLazy                      } ,
		futures_timer   :: { Delay                                       } ,
		parking_lot     :: { Mutex                                       } ,
		async_nursery   :: { NurseExt, Nursery                           } ,
		async_executors :: { SpawnHandle, SpawnHandleExt                 } ,

		std ::
		{
			fmt                                      ,
			convert      :: { TryFrom              } ,
			collections  :: { HashMap, VecDeque    } ,
			sync         :: { Arc                  } ,
			hash         :: { Hasher               } ,
			time         :: { Duration             } ,
			sync::atomic :: { AtomicI64, Ordering  } ,
			pin          :: { Pin                  } ,
			task         :: { Poll, Context, Waker } ,
			future       :: { Future               } ,
			num          :: { NonZeroUsize         } ,
			ops          :: { DerefMut             } ,
		},


		futures ::
		{
			prelude :: { Stream, Sink                } ,
			channel :: { oneshot                     } ,
			future  :: { FutureExt, TryFutureExt     } ,
			sink    :: { SinkExt                     } ,
			stream  :: { StreamExt                   } ,
			task    :: { SpawnError } ,
			lock    :: { Mutex as FutMutex           } ,
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
		pretty_assertions :: { assert_eq             } ,
		futures           :: { executor::block_on    } ,
		futures_test      :: { task::new_count_waker } ,
	};
}
