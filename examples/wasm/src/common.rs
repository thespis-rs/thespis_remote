pub use
{
	thespis               :: { *              } ,
	thespis_impl          :: { *              } ,
	thespis_impl_remote   :: { *              } ,
	wasm_bindgen::prelude :: { *              } ,
	wasm_websocket_stream :: { *              } ,
	log                   :: { *              } ,
	tokio_serde_cbor      :: { Codec          } ,
	serde   :: { Serialize, Deserialize } ,

	futures::
	{
		channel :: { mpsc                                                                    } ,
		compat  :: { Compat01As03Sink, Stream01CompatExt, Sink01CompatExt, Future01CompatExt } ,
		prelude :: { StreamExt                                                               } ,
		future  :: { FutureExt                                                               } ,
	},

	tokio        ::
	{
		prelude :: { Stream as TokStream, stream::{ SplitStream as TokSplitStream, SplitSink as TokSplitSink } } ,
		codec   :: { Decoder, Framed                                                                           } ,
	},
};


pub type TheSink = Compat01As03Sink<TokSplitSink<Framed<WsStream, MulServTokioCodec<MS>>>, MS> ;
pub type MS      = MultiServiceImpl<ServiceID, ConnID, Codecs>                                  ;
pub type MyPeer  = Peer<TheSink, MS>                                                            ;


#[ derive( Serialize, Deserialize ) ]
//
pub struct Ping(pub usize);

impl Message for Ping { type Return = usize; }

/// Actor
//
#[ derive( Actor ) ]
//
pub struct MyActor { pub count: usize }



/// Handler for `Ping` message
//
impl Handler<Ping> for MyActor
{
	fn handle( &mut self, msg: Ping ) -> ReturnNoSend<<Ping as Message>::Return>
	{
		Box::pin( async move
		{
			self.count += msg.0;
			self.count
		})
	}
}




service_map!
(
	namespace:     remotes ;
	peer_type:     MyPeer  ;
	multi_service: MS      ;
	services     : Ping    ;
);


