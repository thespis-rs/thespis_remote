//! # Thespis reference implementation
//!
//! ## Cargo Features
//!
//! - tokio: makes the tokio executor available. enabled by default.
//!

#![ feature
(
	arbitrary_self_types   ,
	async_await            ,
	await_macro            ,
	box_into_pin           ,
	box_patterns           ,
	box_syntax             ,
	core_intrinsics        ,
	decl_macro             ,
	never_type             ,
	nll                    ,
	optin_builtin_traits   ,
	re_rebalance_coherence ,
	specialization         ,
	todo_macro             ,
	trait_alias            ,
	try_trait              ,
	unboxed_closures       ,
)]

mod peer              ;
mod multi_service     ;
mod service_map_macro ;

pub use
{
	peer              :: * ,
	multi_service     :: * ,
	service_map_macro :: * ,
};


// needed for macro
//
pub mod external_deps
{
	pub use once_cell      ;
	pub use futures        ;
	pub use thespis        ;
	pub use thespis_remote ;
	pub use thespis_impl   ;
	pub use serde_cbor     ;
	pub use serde          ;
	pub use failure        ;
}



// Import module. Avoid * imports here. These are all the foreign names that exist throughout
// the crate. The must all be unique.
// Separate use imports per enabled features.
//
mod import
{
	pub use
	{
		failure        :: { Fail, bail, err_msg, AsFail, Context as FailContext, Backtrace, ResultExt } ,
		thespis        :: { *                                           } ,
		thespis_remote :: { *                                           } ,
		thespis_impl   :: { Addr, Receiver, runtime::rt                 } ,
		log            :: { *                                           } ,
		once_cell      :: { unsync::OnceCell, unsync::Lazy, unsync_lazy } ,
		byteorder   :: { LittleEndian, ReadBytesExt, WriteBytesExt           } ,
		bytes       :: { Bytes, BytesMut, Buf, BufMut, IntoBuf               } ,
		num_traits  :: { FromPrimitive, ToPrimitive                          } ,
		num_derive  :: { FromPrimitive, ToPrimitive                          } ,
		rand        :: { Rng                                                 } ,
		std         :: { hash::{ BuildHasher, Hasher }, io::Cursor, any::Any } ,
		twox_hash   :: { RandomXxHashBuilder, XxHash                         } ,
		futures     :: { future::RemoteHandle                                } ,
		pharos      :: { Pharos, Observable                                  } ,
		serde       :: { Serialize, Deserialize, de::DeserializeOwned        } ,

		std ::
		{
			fmt                                                       ,
			cell        :: { RefCell                                } ,
			convert     :: { TryFrom, TryInto                       } ,
			future      :: { Future                                 } ,
			marker      :: { PhantomData                            } ,
			ops         :: { Try, DerefMut                          } ,
			pin         :: { Pin                                    } ,
			rc          :: { Rc                                     } ,
			sync        :: { Arc, atomic::{ AtomicUsize, Ordering } } ,
			collections :: { HashMap                                } ,

		},


		futures ::
		{
			prelude :: { Stream, StreamExt, Sink, SinkExt                                         } ,
			channel :: { oneshot, mpsc                                                            } ,
			future  :: { FutureExt, TryFutureExt                                                  } ,
			task    :: { Spawn, SpawnExt, LocalSpawn, LocalSpawnExt, Context as TaskContext, Poll } ,

			executor::
			{
				LocalPool    as LocalPool03    ,
				LocalSpawner as LocalSpawner03 ,
				ThreadPool   as ThreadPool03   ,
			},
		},
	};


	#[ cfg( feature = "tokio" ) ]
	//
	pub use
	{
		tokio :: { await as await01, prelude::{ AsyncRead as TokioAsyncR, AsyncWrite as TokioAsyncW } } ,
		tokio :: { codec::{ Decoder, Encoder, Framed, FramedParts, FramedRead, FramedWrite } },
	};


	#[ cfg(test) ]
	//
	pub use
	{
		pretty_assertions::{ assert_eq, assert_ne } ,
	};
}
