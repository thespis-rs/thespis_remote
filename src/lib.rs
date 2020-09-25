//! # Thespis reference implementation
//!
#![ doc    ( html_root_url = "https://docs.rs/thespis_remote_impl"       ) ]
#![ deny   ( /*missing_docs,*/ bare_trait_objects                        ) ]
#![ forbid ( unsafe_code                                                 ) ]
#![ allow  ( clippy::suspicious_else_formatting, clippy::type_complexity ) ]

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


pub mod peer              ;
    mod relay_map         ;
    mod service_handler   ;
    mod service_map       ;
    mod service_map_macro ;
pub mod thes_wf           ;
pub mod wire_format       ;

pub use
{
	thes_wf           :: * ,
	peer              :: * ,
	relay_map         :: * ,
	service_handler   :: * ,
	service_map       :: * ,
	service_map_macro :: * ,
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
		async_executors :: { SpawnHandle, SpawnHandleExt, JoinHandle   } ,
		async_nursery   :: { NurseExt, Nursery, NurseryStream          } ,
		futures_timer   :: { Delay                                     } ,
		log             :: { *                                         } ,
		once_cell       :: { sync::Lazy as SyncLazy                    } ,
		parking_lot     :: { Mutex                                     } ,
		pharos          :: { Pharos, Observable, ObserveConfig, Events } ,
		rand            :: { Rng                                       } ,
		serde           :: { Serialize, Deserialize                    } ,
		thespis         :: { *                                         } ,
		thespis_impl    :: { Addr, ThesErr                             } ,
		twox_hash       :: { XxHash64                                  } ,

		std ::
		{
			collections  :: { HashMap, VecDeque                 } ,
			convert      :: { TryFrom, TryInto                  } ,
			fmt                                                   ,
			io                                                    ,
			future       :: { Future                            } ,
			hash         :: { Hasher                            } ,
			marker       :: { PhantomData                       } ,
			num          :: { NonZeroUsize                      } ,
			ops          :: { DerefMut                          } ,
			pin          :: { Pin                               } ,
			sync         :: { Arc                               } ,
			sync::atomic :: { AtomicI64, AtomicU64, Ordering::* } ,
			task         :: { Poll, Context, Waker              } ,
			time         :: { Duration                          } ,
		},


		futures ::
		{
			channel :: { oneshot           } ,
			future  :: { FutureExt         } ,
			lock    :: { Mutex as FutMutex } ,
			prelude :: { Stream, Sink      } ,
			sink    :: { SinkExt           } ,
			stream  :: { StreamExt         } ,
			io      :: { AsyncReadExt      } ,
			AsyncRead  as FutAsyncRead  ,
			AsyncWrite as FutAsyncWrite ,
		},
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
