use crate::{ import::*, ThesWF, WireErr };

pub struct Encoder<T>
{
	out_bytes: T,
}




impl<T> Sink<ThesWF> for Encoder<T>

	where T: FutAsyncWrite

{
	type Error = WireErr;


	fn poll_ready( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll< Result<(), Self::Error> >
	{

	}


	fn start_send( mut self: Pin<&mut Self>, msg: ThesWF ) -> Result<(), Self::Error>
	{

	}


	fn poll_flush( mut self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<Result<(), Self::Error>>
	{

	}


	fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>
	{
	}
}
