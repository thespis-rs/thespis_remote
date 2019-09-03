//! # Thespis reference implementation
//!
//! ## Cargo Features
//!
//! - tokio: makes the tokio executor available. enabled by default.

#![deny(bare_trait_objects)]
#![forbid(unsafe_code)]

#![ feature
(
	arbitrary_self_types   ,
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

pub mod peer              ;
pub mod multi_service     ;
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
	pub use async_runtime  ;
	pub use failure        ;
	pub use futures        ;
	pub use log            ;
	pub use once_cell      ;
	pub use serde_cbor     ;
	pub use serde          ;
	pub use thespis        ;
	pub use thespis_remote ;
	pub use thespis_impl   ;
}


// Don't know exactly where to put this yet. It's useful for debugging.
//
pub fn ashex( buf: &[u8] ) -> String
{
	let mut f = String::new();

	for byte in buf
	{
		std::fmt::write( &mut f, format_args!( "{:02x}", byte ) ).expect( "Create hex string from slice" )
	}

	f
}



// Import module. Avoid * imports here. These are all the foreign names that exist throughout
// the crate. They must all be unique.
// Separate use imports per enabled features.
//
mod import
{
	pub use
	{
		async_runtime  :: { rt                                                                        } ,
		failure        :: { Fail, bail, err_msg, AsFail, Context as FailContext, Backtrace, ResultExt } ,
		thespis        :: { *                                                                         } ,
		thespis_remote :: { *                                                                         } ,
		thespis_impl   :: { Addr, Receiver                                                            } ,
		log            :: { *                                                                         } ,
		once_cell      :: { unsync::OnceCell, unsync::Lazy                                            } ,
		byteorder      :: { LittleEndian, ReadBytesExt, WriteBytesExt                                 } ,
		bytes          :: { Bytes, BytesMut, Buf, BufMut, IntoBuf                                     } ,
		num_traits     :: { FromPrimitive, ToPrimitive                                                } ,
		num_derive     :: { FromPrimitive, ToPrimitive                                                } ,
		rand           :: { Rng                                                                       } ,
		std            :: { hash::{ BuildHasher, Hasher }, io::Cursor, any::Any                       } ,
		twox_hash      :: { RandomXxHashBuilder, XxHash                                               } ,
		futures        :: { future::RemoteHandle                                                      } ,
		pharos         :: { Pharos, Observable, UnboundedObservable                                   } ,
		serde          :: { Serialize, Deserialize, de::DeserializeOwned                              } ,
		futures_codec  :: { Encoder, Decoder, Framed, FramedRead, FramedWrite                         } ,

		std ::
		{
			fmt                                                       ,
			any         :: { TypeId                                 } ,
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
			prelude :: { Stream, Sink                                                             } ,
			channel :: { oneshot, mpsc                                                            } ,
			future  :: { FutureExt, TryFutureExt                                                  } ,
			task    :: { Spawn, SpawnExt, LocalSpawn, LocalSpawnExt, Context as TaskContext, Poll } ,
			sink    :: { SinkExt                                                                  } ,
			stream  :: { StreamExt                                                                } ,

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
		tokio :: { prelude::{ AsyncRead as TokioAsyncR, AsyncWrite as TokioAsyncW }          } ,
		// tokio :: { codec::{ Decoder, Encoder, Framed, FramedParts, FramedRead, FramedWrite } } ,
	};


	#[ cfg(test) ]
	//
	pub use
	{
		pretty_assertions::{ assert_eq, assert_ne } ,
	};
}
