use crate :: { import::*, single_thread::*, runtime::rt };


// TODO: Ideas for improvement. Create a struct RawAddress, which allows to create other addresses from.
//       Do not hold anything in the mailbox struct. just PhantomData<A>.
//       In method start, create the channel. give msgs to start_fut, and create rawaddress from handle.
//       return rawaddress to user.
//
//       Currently there is no way for the actor to decide to stop itself. Also, currently we give nothing
//       to the actor in the started or stopped method (like a context, a RawAddress).
//
pub struct Inbox<A> where A: Actor
{
	handle: mpsc::UnboundedSender  <Box< dyn Envelope<A> >> ,
	msgs  : mpsc::UnboundedReceiver<Box< dyn Envelope<A> >> ,
}

impl<A> Inbox<A> where A: Actor
{
	pub fn new() -> Self
	{
		let (handle, msgs) = mpsc::unbounded();

		Self { handle, msgs }
	}

	pub fn sender( &self ) -> mpsc::UnboundedSender<Box< dyn Envelope<A> >>
	{
		self.handle.clone()
	}
}




impl<A> Mailbox<A> for Inbox<A> where A: Actor
{
	fn start( self, actor: A ) -> ThesRes<()>
	{
		rt::spawn_pinned( self.start_fut( actor ) )
	}


	fn start_fut( self, mut actor: A ) -> Pin<Box< dyn Future<Output = ()> >>
	{
		async move
		{
			// TODO: Clean this up...
			// We need to drop the handle, otherwise the channel will never close and the program will not
			// terminate. Like this when the user drops all their handles, this loop will automatically break.
			//
			let Inbox{ mut msgs, handle } = self;
			drop( handle );

			await!( actor.started() );

			loop
			{
				trace!( "mailbox: starting loop, if we don't get a message, should register waker" );

				match await!( msgs.next() )
				{
					Some( envl ) => { trace!( "mb: envelope handle" ); await!( envl.handle( &mut actor ) ); trace!( "mb: envelope handle finished, ready to loop" );}
					None         => { trace!( "break from mailbox"  ); break;                               }
				}
			}

			await!( actor.stopped() );
			trace!( "Mailbox stopped actor" );

		}.boxed()
	}
}


impl<A> Default for Inbox<A> where A: Actor
{
	fn default() -> Self { Self::new() }
}
