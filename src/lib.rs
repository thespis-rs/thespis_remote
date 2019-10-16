//! # Thespis reference implementation
//!
//! ## Cargo Features
//!
//! - tokio: makes the tokio executor available. enabled by default.

#![deny(bare_trait_objects)]
#![forbid(unsafe_code)]

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
#[ doc( hidden ) ]
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


/// Print the contents of a buffer as a hex string.
/// Don't know exactly where to put this yet. It's useful for debugging.
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
	pub(crate) use
	{
		async_runtime  :: { rt                                                  } ,
		failure        :: { ResultExt                                           } ,
		thespis        :: { *                                                   } ,
		thespis_remote :: { *                                                   } ,
		thespis_impl   :: { Addr                                                } ,
		log            :: { *                                                   } ,
		byteorder      :: { LittleEndian, ReadBytesExt, WriteBytesExt           } ,
		bytes          :: { Bytes, BytesMut, BufMut                             } ,
		num_traits     :: { FromPrimitive, ToPrimitive                          } ,
		num_derive     :: { FromPrimitive, ToPrimitive                          } ,
		rand           :: { Rng                                                 } ,
		std            :: { hash::{ BuildHasher, Hasher }, io::Cursor, any::Any } ,
		twox_hash      :: { RandomXxHashBuilder, XxHash                         } ,
		futures        :: { future::RemoteHandle                                } ,
		pharos         :: { Pharos, Observable, ObserveConfig, Events           } ,
		serde          :: { Serialize, Deserialize                              } ,

		std ::
		{
			fmt                            ,
			any         :: { TypeId      } ,
			convert     :: { TryFrom     } ,
			marker      :: { PhantomData } ,
			collections :: { HashMap     } ,

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
		pretty_assertions::{ assert_eq, assert_ne } ,
	};
}
