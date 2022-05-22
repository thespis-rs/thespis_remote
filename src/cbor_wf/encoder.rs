use crate::{ import::*, ThesWF, WireErr, WireFormat };


/// Serializes the ThesWf format onto a stream of bytes.
//
#[ derive(Debug) ]
//
pub struct Encoder<T>
{
	out_bytes: T                         ,
	buffer   : Option< (ThesWF, usize) > ,
	max_size : usize                     ,
}


impl<T> Encoder<T>
{
	/// Create a new encoder.
	//
	pub fn new( out_bytes: T, max_size: usize ) -> Self
	{
		Self
		{
			out_bytes    ,
			max_size     ,
			buffer: None ,
		}
	}
}


impl<T> Sink<ThesWF> for Encoder<T>

	where T: AsyncWrite + Unpin

{
	type Error = WireErr;


	fn poll_ready( self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Result<(), Self::Error> >
	{
		self.poll_flush( cx )
	}


	fn start_send( mut self: Pin<&mut Self>, msg: ThesWF ) -> Result<(), Self::Error>
	{
		if self.buffer.is_some()
		{
			panic!( "call `poll_ready` before start_send" )
		}

		let len = msg.len() as usize;

		if len > self.max_size
		{
			return Err( WireErr::MessageSizeExceeded
			{
				context: "encoder start_send".to_string(),
				size   : len,
				max_size: self.max_size,
			});
		}

		self.buffer = Some( (msg, 0) );

		Ok(())
	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		loop { match self.buffer.take()
		{
			None => return Poll::Ready( Ok(()) ),

			Some( (msg, mut pos) ) =>
			{
				match Pin::new( &mut self.out_bytes ).poll_write( cx, &msg.as_buf()[pos..] )
				{
					Poll::Pending =>
					{
						self.buffer = Some( (msg, pos) );
						return Poll::Pending;
					}


					Poll::Ready( Ok(0) ) => // TODO: what to do? normally means connection closed.
					{
						return Err( WireErr::from( io::Error::from( io::ErrorKind::ConnectionAborted ) )).into();
					}


					Poll::Ready( Ok(x) ) =>
					{
						pos += x;

						// we wrote all
						//
						if pos == msg.as_buf().len()
						{
							return Ok(()).into()
						}

						self.buffer = Some( (msg, pos) );
					}


					Poll::Ready( Err(e) ) =>
					{
						// TODO: are we still operational after an error?
						//
						return Err( WireErr::from(e) ).into()
					}
				}
			}
		}}
	}


	fn poll_close( self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{
		// TODO: do we need to keep track of a closed state and refuse new items?
		//
		self.poll_flush( cx )
	}
}
