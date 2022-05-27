use
{
	crate     :: { CborWF                      } ,
	super     :: { *                           } ,
	byteorder :: { ReadBytesExt, LittleEndian  } ,
	std       :: { future::Future              } ,
	futures   :: { io::AsyncReadExt            } ,
};



/// Decodes a stream of bytes into a stream of CborWF messages.
/// T = Transport layer type.
//
#[ allow( clippy::type_complexity ) ]
//
pub struct Decoder<T>
{
	byte_stream: Option<T> ,

	/// Future for the first phase of getting the length of the wire.
	//
	get_len    : Option< Pin<Box< dyn Future<Output=(T, io::Result<[u8;LEN_LEN]>)> + Send >> > ,

	/// Future for the second phase of getting the actual message of the wire.
	//
	get_msg    : Option< Pin<Box< dyn Future<Output=(T, io::Result<Vec<u8>     >)> + Send >> > ,
	closed     : bool                                                                          ,
	max_size   : usize                                                                         ,
}


impl<T> Decoder<T>
{
	/// Create a new decoder.
	//
	pub fn new( byte_stream: T, max_size: usize ) -> Self
	{
		Self
		{
			byte_stream: Some( byte_stream ) ,
			get_len    : None                ,
			get_msg    : None                ,
			closed     : false               ,
			max_size                         ,
		}
	}
}


impl<T: fmt::Debug> fmt::Debug for Decoder<T>
{
	fn fmt( &self, fmt: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		fmt.debug_struct( "thes_wf::decoder" )

			.field( "byte_stream", &self.byte_stream                                                   )
			.field( "get_len"    , &self.get_len.as_ref().map( |_| "future getting the length field" ) )
			.field( "get_msg"    , &self.get_len.as_ref().map( |_| "future getting the message"      ) )
			.field( "closed"     , &self.closed                                                        )
			.field( "max_size"   , &self.max_size                                                      )

		.finish()
	}
}


impl<T> Stream for Decoder<T>

	where T: AsyncRead + Unpin + Send + 'static
{
	type Item = Result<CborWF, WireErr>;


	/// There is 2 steps to this operation. First we read the length header which tells us how big
	/// the rest of the message is. Next we read the amount of bytes speficied in length and try to
	/// deserialize into CborWF.
	///
	/// Since each of those fases can return pending and must be resumed afterwards, we store them
	/// as a future to resume on the next poll.
	///
	/// TODO: get rid of the boxed futures and just store the relevant information in an enum to
	/// resume the operation.
	//
	fn poll_next( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Option<Self::Item> >
	{
		if self.closed
		{
			return Poll::Ready( None );
		}


		loop
		{
			// We are getting the serialized user message.
			//
			if let Some(mut get_msg) = self.get_msg.take()
			{
				match get_msg.as_mut().poll( cx )
				{
					Poll::Pending =>
					{
						self.get_msg = Some( get_msg );
						return Poll::Pending;
					}

					Poll::Ready( (transport, Err(e)) ) =>
					{
						self.closed = true;
						self.byte_stream = Some(transport);

						match e.kind()
						{
							io::ErrorKind::UnexpectedEof => return Poll::Ready( None ),
							_                            => return Some(Err( WireErr::from(e) )).into(),
						}
					}

					Poll::Ready( (transport, Ok(all)) ) =>
					{
						self.byte_stream = Some(transport);
						let thes_wf = CborWF::try_from( all )?;

						return Poll::Ready( Some(Ok( thes_wf )) );
					}
				}
			}


			// We are getting the length.
			//
			if let Some(mut get_len) = self.get_len.take()
			{
				match get_len.as_mut().poll( cx )
				{
					Poll::Pending =>
					{
						self.get_len = Some( get_len );
						return Poll::Pending;
					}

					Poll::Ready( (transport, Err(e)) ) =>
					{
						self.closed = true;
						self.byte_stream = Some(transport);

						match e.kind()
						{
							io::ErrorKind::UnexpectedEof => return Poll::Ready( None ),
							_                            => return Some(Err( WireErr::from(e) )).into(),
						}
					}

					Poll::Ready( (mut transport, Ok(buf)) ) =>
					{
						let len: usize = buf[ 0..LEN_LEN ].as_ref().read_u64::<LittleEndian>().unwrap().try_into().unwrap();

						debug_assert!( len >= LEN_HEADER );

						if len > self.max_size
						{
							let err = WireErr::MessageSizeExceeded
							{
								size    : len                          ,
								max_size: self.max_size                ,
								context : "CborWF Decoder".to_string() ,
							};

							return Poll::Ready( Some(Err( err )) );
						}

						// Create a zeroed buffer of the size of the entire message.
						// TODO: check the perf difference with an unzeroed buffer.
						//
						let mut all = vec![0u8;len];

						// put the length in the new buffer.
						//
						all[ 0..LEN_LEN ].copy_from_slice( &buf );

						self.get_msg = Some( async move
						{
							let res = transport.read_exact( &mut all[LEN_LEN..] ).await.map( |_| all );

							(transport, res)

						}.boxed() );
					}
				}
			}

			// nothing in progress.
			//
			else
			{
				// create get_len.
				//
				let mut transport = self.byte_stream.take().unwrap();

				self.get_len = Some( async move
				{
					let mut buf = [0u8;LEN_LEN];
					let     res = transport.read_exact( &mut buf ).await.map( |_| buf );

					(transport, res)

				}.boxed() );
			}
		}
	}
}
