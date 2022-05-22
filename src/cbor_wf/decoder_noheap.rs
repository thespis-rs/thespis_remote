use
{
	crate     :: { import::*, ThesWF, WireErr, cbor_wf::{ LEN_LEN, LEN_HEADER }  } ,
	byteorder :: { ReadBytesExt, LittleEndian  } ,
	std       :: { io::{ Write, Cursor }       } ,
};



/// Decoder that does not heap allocate.
//
#[ derive(Debug) ]
//
pub struct DecoderNoHeap<T>
{
	byte_stream : T                         ,
	in_progress : Option< Cursor<Vec<u8>> > ,
	closed      : bool                      ,
	max_size    : usize                     ,
}


impl<T> DecoderNoHeap<T>
{
	/// Create a new decoder.
	//
	pub fn new( byte_stream: T, max_size: usize ) -> Self
	{
		Self
		{
			byte_stream         ,
			max_size            ,
			in_progress : None  ,
			closed      : false ,
		}
	}
}



impl<T> Stream for DecoderNoHeap<T>

	where T: AsyncRead + Unpin
{
	type Item = Result<ThesWF, WireErr>;


	// #[log_derive::logfn(Debug)]
	//
	fn poll_next( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Option<Self::Item> >
	{
		if self.closed
		{
			debug!("DecoderNoHeap::poll_next, returning None" );
			return Poll::Ready( None );
		}


		let mut in_progress = self.in_progress.take().unwrap_or_else( || Cursor::new( vec![0u8;LEN_LEN] ) );

		loop
		{
			// Order of the patterns matters.
			// TODO: conversion needs to be checked. It may be best to limit to usize::MAX.
			//
			match usize::try_from( in_progress.position() ).unwrap()
			{
				// We don't yet have the length, that is less than one u64. Continue trying to read just the length.
				//
				pos if pos < LEN_LEN =>
				{
					debug!( "decoder_noheap: read less than LEN_LEN" );

					match Pin::new( &mut self.byte_stream ).poll_read( cx, &mut in_progress.get_mut()[pos..] )
					{
						Poll::Pending =>
						{
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}


						Poll::Ready(Ok( read )) =>
						{
							in_progress.set_position( in_progress.position() + read as u64 );
							continue;
						}


						Poll::Ready( Err(e) ) =>
						{
							// TODO: is there some errors that would allow us to continue afterwards?
							//
							self.closed = true;

							return Some(Err( WireErr::from(e) )).into();
						}
					}
				}


				// We read the length u64, we can now try to read the rest of the message in an exactly sized buffer.
				//
				pos if  pos == LEN_LEN  &&  in_progress.get_ref().len() == LEN_LEN  =>
				{
					// TODO: this can truncate.
					//
					let len: usize = in_progress.get_ref()[ 0..LEN_LEN ].as_ref().read_u64::<LittleEndian>().unwrap().try_into().unwrap();

					if len > self.max_size
					{
						let err = WireErr::MessageSizeExceeded
						{
							size    : len                          ,
							max_size: self.max_size                ,
							context : "ThesWF Decoder".to_string() ,
						};

						return Poll::Ready( Some(Err( err )) );
					}

					// if we havent allocated to buffer yet on a previous run:
					//
					// TODO: do we get perf wins if we use debug_assert! here and get_unchecked_mut below?
					//
					assert!( len >= LEN_HEADER );

					// Create a zeroed buffer of the size of the entire message.
					// TODO: check the perf difference with an unzeroed buffer.
					//
					let mut tmp = io::Cursor::new( vec![0u8;len] );

					// put the length in the new buffer.
					//
					tmp.write_all( &in_progress.get_ref()[0..LEN_LEN] )?;

					in_progress = tmp;

					continue;
				}


				// We read a partial message.
				//
				pos =>
				{
					let len     = in_progress.get_ref()[ 0..LEN_LEN ].as_ref().read_u64::<LittleEndian>().unwrap() as usize;
					let to_read = len - pos;

					match Pin::new( &mut self.byte_stream ).poll_read( cx, &mut in_progress.get_mut()[ pos..len ] )
					{
						Poll::Pending =>
						{
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}

						Poll::Ready(Ok( read )) if read < to_read =>
						{
							in_progress.set_position( (pos + read) as u64 );
							continue;
						}

						// We have a full item.
						//
						Poll::Ready(Ok( read )) if read == to_read =>
						{
							in_progress.set_position( in_progress.position() + read as u64 );
							debug_assert_eq!( len as u64, in_progress.position() );

							let thes_wf = ThesWF::try_from( in_progress.into_inner() )?;

							return Poll::Ready( Some(Ok( thes_wf )) );
						}

						_ => unreachable!( "read more bytes than buffer size" ),
					}
				}
			}
		}
	}
}
