use { crate :: { import::* }};



// An async counter that keeps track of the number of available slots for processing. When the counter
// is <= 0, this will return Pending and can be awaited. When the counter becomes >= 1, the last waker
// that awaits it will be woken up. Thus you should only use it from within one context.
//
// TODO: Review and document behavior. It uses a vecdeque, so it stores more than one waker...
//
// Currently used only for incoming calls, not incoming sends.
//
#[ derive( Debug, Actor ) ]
//
pub struct BackPressure
{
	// The total number of available slots.
	//
	available: Arc<AtomicU64>              ,
	wakers   : Arc<Mutex<VecDeque<Waker>>> ,
	future   : FutMutex<BPInner>           ,
}


impl BackPressure
{
	pub fn new( slots: u64 ) -> Self
	{
		let wakers    = Arc::new( Mutex::new( VecDeque::new() ) );
		let available = Arc::new( AtomicU64::from( slots )      );

		let w = wakers.clone();
		let a = available.clone();

		Self
		{
			available: a,
			wakers   : w,
			future   : FutMutex::new( BPInner{ available, wakers } ) ,
		}
	}


	pub fn add_slots( &self, num: NonZeroUsize )
	{
		let num = num.get() as u64;

		let old = self.available.fetch_add( num, SeqCst );

		if old == 0
		{
			if let Some(w) = self.wakers.lock().pop_front()
			{
				w.wake();
			}
		}
	}


	pub fn available( &self ) -> u64
	{
		self.available.load( SeqCst )
	}


	pub async fn wait( &self )
	{
		self.future.lock().await.deref_mut().await
	}
}





struct BPInner
{
	available: Arc<AtomicU64>,
	wakers   : Arc<Mutex<VecDeque<Waker>>> ,
}



impl Future for BPInner
{
	type Output = ();

	fn poll( self: Pin<&mut Self>, cx: &mut Context<'_> ) -> Poll<()>
	{
		let mut woke = false;

		if self.available.load( SeqCst ) > 0
		{
			self.available.fetch_sub( 1, SeqCst );
			woke = true;
		}

		while self.available.load( SeqCst ) > 0
		{
			if let Some(w) = self.wakers.lock().pop_front()
			{
				woke = true;
				w.wake();
				self.available.fetch_sub( 1, SeqCst );
			}
		}

		match woke
		{
			true  => Poll::Ready(()),

			false =>
			{
				self.wakers.lock().push_back( cx.waker().clone() );
				Poll::Pending
			}
		}
	}
}



#[cfg(test)]
//
mod tests
{
	// What's tested:
	//
	// ✔ basic waking up when adding slots.
	// ✔ remove slots when polled and returning ready.
	//
	use crate::{ import::{ *, assert_eq }, peer::BackPressure };

	#[test]
	//
	fn wakeup() { block_on( async
	{
		let (waker, count ) = new_count_waker();

		let mut cx = Context::from_waker( &waker );

		let bp = BackPressure::new( 0 );

			assert_eq!( Box::pin( bp.wait() ).as_mut().poll( &mut cx ), Poll::Pending );
			assert_eq!( count         , 0 );
			assert_eq!( bp.available(), 0 );

		bp.add_slots( NonZeroUsize::new(1).unwrap() );

			assert_eq!( count, 1 );
			assert_eq!( Box::pin( bp.wait() ).as_mut().poll( &mut cx  ), Poll::Ready(()) );
	})}


	#[test]
	//
	fn wakeup2() { block_on( async
	{
		let (waker , count ) = new_count_waker();
		let (waker2, count2) = new_count_waker();

		let mut cx  = Context::from_waker( &waker  );
		let mut cx2 = Context::from_waker( &waker2 );

		let bp = BackPressure::new( 0 );

			assert_eq!( Box::pin( bp.wait() ).as_mut().poll( &mut cx  ), Poll::Pending );
			assert_eq!( Box::pin( bp.wait() ).as_mut().poll( &mut cx2 ), Poll::Pending );

			assert_eq!( count         , 0 );
			assert_eq!( count2        , 0 );
			assert_eq!( bp.available(), 0 );

		// both should be woken up
		//
		bp.add_slots( NonZeroUsize::new(2).unwrap() );

			assert_eq!( count , 1 );

			assert_eq!( Box::pin( bp.wait() ).as_mut().poll( &mut cx  ), Poll::Ready(()) );
			assert_eq!( count2, 1 );

			assert_eq!( Box::pin( bp.wait() ).as_mut().poll( &mut cx  ), Poll::Ready(()) );

			// We don't remove slots just by awaiting, so both are still there.
			//
			assert_eq!( bp.available(), 2 );

	})}
}


