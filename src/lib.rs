//! # Thespis reference implementation
//!
//! ## Cargo Features
//!
//! - tokio: makes the tokio executor available. enabled by default.



#![ doc    ( html_root_url = "https://docs.rs/thespis_remote_impl" ) ]
#![ deny   ( missing_docs, bare_trait_objects                      ) ]
#![ forbid ( unsafe_code                                           ) ]
#![ allow  ( clippy::suspicious_else_formatting                    ) ]

#![ warn
(
	missing_debug_implementations ,
	missing_docs                  ,
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
pub mod multi_service     ;
    mod service_map_macro ;
    mod service_map       ;

pub use
{
	error             :: * ,
	peer              :: * ,
	multi_service     :: * ,
	service_map       :: * ,
	service_map_macro :: * ,
};


// needed for macro
//
#[ doc( hidden ) ]
//
pub mod external_deps
{
	pub use async_runtime  ;
	pub use futures        ;
	pub use log            ;
	pub use once_cell      ;
	pub use serde_cbor     ;
	pub use serde          ;
	pub use thespis        ;
	pub use thespis_impl   ;
}



// Import module. Avoid * imports here. These are all the foreign names that exist throughout
// the crate. They must all be unique.
// Separate use imports per enabled features.
//
mod import
{
	pub(crate) use
	{
		async_runtime  :: { rt                                        } ,
		thespis        :: { *                                         } ,
		thespis_impl   :: { Addr, ThesErr                             } ,
		log            :: { *                                         } ,
		byteorder      :: { LittleEndian, ReadBytesExt, WriteBytesExt } ,
		bytes          :: { Bytes, BytesMut, BufMut                   } ,
		rand           :: { Rng                                       } ,
		std            :: { hash::{ Hasher }, any::Any                } ,
		twox_hash      :: { XxHash                                    } ,
		futures        :: { future::RemoteHandle                      } ,
		pharos         :: { Pharos, Observable, ObserveConfig, Events } ,
		serde          :: { Serialize, Deserialize                    } ,
		thiserror      :: { Error                                     } ,


		std ::
		{
			fmt                            ,
			any         :: { TypeId      } ,
			convert     :: { TryFrom     } ,
			collections :: { HashMap     } ,
			sync        :: { Arc         } ,
		},


		futures ::
		{
			prelude :: { Stream, Sink } ,
			channel :: { oneshot      } ,
			future  :: { FutureExt    } ,
			sink    :: { SinkExt      } ,
			stream  :: { StreamExt    } ,
		},
	};


	#[ cfg( feature = "tokio" ) ]
	//
	pub use
	{
		tokio :: { prelude::{ AsyncRead as TokioAsyncR, AsyncWrite as TokioAsyncW }          } ,
		tokio :: { codec::{ Decoder, Encoder, Framed, FramedParts, FramedRead, FramedWrite } } ,
	};


	#[ cfg(not( feature = "tokio" )) ]
	//
	pub(crate) use
	{
		futures_codec :: { Encoder, Decoder } ,
	};


	#[ cfg(test) ]
	//
	pub(crate) use
	{
		pretty_assertions :: { assert_eq } ,
	};
}
