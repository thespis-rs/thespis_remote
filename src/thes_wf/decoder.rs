use
{
	crate     :: { import::*, ThesWF } ,
	super     :: { *                           } ,
	byteorder :: { ReadBytesExt, LittleEndian  } ,
	std       :: { io::{ Write, Cursor }       } ,
};



pub struct Decoder<T>
{
	byte_stream: T,
	in_progress: Option< Cursor<Vec<u8>> >,
	closed: bool,
}


impl<T> Decoder<T>
{
	pub fn new( byte_stream: T ) -> Self
	{
		Self
		{
			byte_stream,
			in_progress: None,
			closed: false,
		}
	}
}




impl<T> Stream for Decoder<T>

	where T: FutAsyncRead + Unpin
{
	type Item = Result<ThesWF, WireErr>;

	fn poll_next( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Option<Self::Item> >
	{
		if self.closed
		{
			return Poll::Ready( None );
		}


		let mut in_progress = self.in_progress.take().unwrap_or_else( || Cursor::new( vec![0u8;LEN_LEN] ) );

		loop
		{
			// Order of the patterns matters.
			// TODO: conversion needs to be checked. It may be best to limit to usize::MAX.
			//
			match TryInto::<usize>::try_into( in_progress.position() ).unwrap()
			{
				// No data so far.
				//
				0 =>
				{
					match Pin::new( &mut self.byte_stream ).poll_read( cx, &mut in_progress.get_ref() )
					{
						Poll::Ready(Ok( read )) if read < LEN_LEN =>
						{
							in_progress.set_position( read.try_into().unwrap() );
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}


						Poll::Ready(Ok( LEN_LEN )) =>
						{
							in_progress.set_position( LEN_LEN.try_into().unwrap() );
							continue;
						}


						Poll::Ready( Err(e) ) =>
						{
							// TODO: is there some errors that would allow us to continue afterwards?
							//
							self.closed = true;

							return Some(Err( WireErr::from(e) )).into();
						}


						_ => unreachable!( "read more bytes than buffer size" ),
					}
				}


				// We read a partial length, that is less than one u64. Continue trying to read just the length.
				//
				pos if pos < LEN_LEN =>
				{
					match Pin::new( &mut self.byte_stream ).poll_read( cx, &mut in_progress.get_ref()[pos..] )
					{
						Poll::Ready(Ok( read )) if read < LEN_LEN =>
						{
							in_progress.set_position( read.try_into().unwrap() );
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}


						Poll::Ready(Ok( LEN_LEN )) =>
						{
							in_progress.set_position( LEN_LEN.try_into().unwrap() );
							continue;
						}


						Poll::Ready( Err(e) ) =>
						{
							// TODO: is there some errors that would allow us to continue afterwards?
							//
							self.closed = true;

							return Some(Err( WireErr::from(e) )).into();
						}


						_ => unreachable!( "read more bytes than buffer size" ),
					}
				}


				// We read the length u64, we can now try to read the rest of the message in an exactly sized buffer.
				//
				pos if pos == LEN_LEN =>
				{
					// TODO: this can truncate.
					//
					let len: usize = in_progress.get_ref()[ 0..LEN_LEN ].as_ref().read_u64::<LittleEndian>().unwrap().try_into().unwrap();
					let to_read    = len - LEN_LEN;

					// if we havent allocated to buffer yet on a previous run:
					//
					if in_progress.get_ref().len() < len
					{
						// TODO: do we get perf wins if we use debug_assert! here and get_unchecked_mut below?
						//
						assert!( len >= LEN_HEADER );

						// Create a zeroed buffer of the size of the entire message.
						// TODO: check the perf difference with an unzeroed buffer.
						//
						let tmp = io::Cursor::new( vec![0u8;len] );

						// put the length in the new buffer.
						//
						tmp.write( &in_progress.get_ref()[0..LEN_LEN] )?;

						in_progress = tmp;
					}


					match Pin::new( &mut self.byte_stream ).poll_read( cx, &mut in_progress.get_ref()[ LEN_LEN..len ] )
					{
						Poll::Pending =>
						{
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}

						Poll::Ready(Ok( read )) if read < to_read =>
						{
							in_progress.set_position( read.try_into().unwrap() );
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}

						// We have a full item.
						//
						Poll::Ready(Ok( to_read )) =>
						{
							debug_assert_eq!( len as u64, in_progress.position() );

							let thes_wf = ThesWF::try_from( in_progress.into_inner() )?;

							return Poll::Ready( Some(Ok( thes_wf )) );
						}

						_ => unreachable!( "read more bytes than buffer size" ),
					}
				}


				// We read a partial message.
				//
				pos =>
				{
					let len     = in_progress.get_ref()[ 0..LEN_LEN ].as_ref().read_u64::<LittleEndian>().unwrap() as usize;
					let to_read = len - pos;

					match Pin::new( &mut self.byte_stream ).poll_read( cx, &mut in_progress.get_ref()[ LEN_LEN..len ] )
					{
						Poll::Pending =>
						{
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}

						Poll::Ready(Ok( read )) if read < to_read =>
						{
							in_progress.set_position( in_progress.position() + read as u64 );
							self.in_progress = Some( in_progress );
							return Poll::Pending;
						}

						// We have a full item.
						//
						Poll::Ready(Ok( to_read )) =>
						{
							debug_assert_eq!( len as u64, in_progress.position() );

							let thes_wf = ThesWF::try_from( in_progress.into_inner() )?;

							return Poll::Ready( Some(Ok( thes_wf )) );
						}

						_ => unreachable!( "read more bytes than buffer size" ),
					}
				}


				// We would have read more than an entire message, but we don't ever create a buffer that is so big,
				// this is an error.
				//
				_pos if _pos as u64 > in_progress.position() => unreachable!( "read more bytes than buffer size 2" ),
			}
		}
	}


	// fn size_hint( &self ) -> (usize, Option<usize>)
	// {
	// 	(0, None)
	// }
}
