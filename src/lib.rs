#![ cfg_attr( nightly, feature(doc_cfg) ) ]
#![ doc = include_str!("../README.md") ]

#![ doc    ( html_root_url = "https://docs.rs/thespis_remote"            ) ]
#![ forbid ( unsafe_code                                                 ) ]
#![ deny   ( /*missing_docs,*/ bare_trait_objects                        ) ]
#![ allow  ( clippy::suspicious_else_formatting, clippy::type_complexity ) ]

#![ warn
(
	anonymous_parameters          ,
	missing_copy_implementations  ,
	missing_debug_implementations ,
	nonstandard_style             ,
	rust_2018_idioms              ,
	single_use_lifetimes          ,
	trivial_casts                 ,
	trivial_numeric_casts         ,
	unreachable_pub               ,
	unused_extern_crates          ,
	unused_qualifications         ,
	variant_size_differences      ,
)]


    mod cbor_wf           ;
pub mod peer              ;
    mod relay_map         ;
    mod pub_sub           ;
    mod service_handler   ;
    mod service_map       ;
    mod service_map_macro ;
pub mod wire_format       ;

pub use
{
	cbor_wf           :: * ,
	peer              :: * ,
	pub_sub           :: * ,
	relay_map         :: * ,
	service_handler   :: * ,
	service_map       :: * ,
	wire_format       :: * ,
};


// needed for macro
//
#[ doc( hidden ) ]
//
pub mod external_deps
{
	pub use futures      ;
	pub use tracing      ;
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
		async_executors :: { SpawnHandle, SpawnHandleExt, JoinHandle             } ,
		tokio::sync     :: { Semaphore, OwnedSemaphorePermit                     } ,
		async_nursery   :: { NurseExt, Nursery, NurseryStream                    } ,
		byteorder       :: { ReadBytesExt, WriteBytesExt, LittleEndian           } ,
		futures_timer   :: { Delay                                               } ,
		tracing         :: { *                                                   } ,
		once_cell       :: { sync::Lazy as SyncLazy                              } ,
		parking_lot     :: { Mutex                                               } ,
		pharos          :: { Pharos, Observe, Observable, ObserveConfig, PharErr } ,
		rand            :: { Rng                                                 } ,
		serde           :: { Serialize, Deserialize                              } ,
		thespis         :: { *                                                   } ,
		thespis_impl    :: { Addr, WeakAddr, ThesErr, Mailbox, DynError          } ,
		twox_hash       :: { XxHash64                                            } ,

		std ::
		{
			collections  :: { HashMap                } ,
			convert      :: { TryFrom, TryInto       } ,
			fmt                                        ,
			io                                         ,
			future       :: { Future                 } ,
			hash         :: { Hasher                 } ,
			marker       :: { PhantomData            } ,
			pin          :: { Pin                    } ,
			sync         :: { Arc                    } ,
			sync::atomic :: { AtomicU64, Ordering::* } ,
			task         :: { Poll, Context          } ,
			time         :: { Duration               } ,
		},


		futures ::
		{
			channel :: { oneshot, mpsc::{ self, UnboundedSender as futUnboundSender } } ,
			future  :: { FutureExt                                                    } ,
			prelude :: { Stream, Sink                                                 } ,
			sink    :: { SinkExt                                                      } ,
			stream  :: { StreamExt, FuturesUnordered                                  } ,
			AsyncRead, AsyncReadExt,
			AsyncWrite,
			pin_mut,
		},
	};


	#[ cfg(test) ]
	//
	pub(crate) use
	{
		pretty_assertions :: { assert_eq } ,
	};
}
