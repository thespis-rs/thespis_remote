use crate::{ *, import::* };

/// PubSub allows a publish-subscribe workflow using for relayed messages.
/// That is you can relay messages to several connected peers.
///
/// This only works for `Sink::send`, not for `Address::call` as we can't
/// decide which subscriber would formulate the response and we could only
/// send one response, so it will simply return an error if the client uses
/// call.
///
/// # Usage
/// Just like [`RelayMap`] this is a [`ServiceMap`] you can register with a [`Peer`].
/// TODO
//
#[ derive(Actor) ]
//
pub struct PubSub<Wf: 'static = ThesWF>
{
	services   : Vec<ServiceID>,
	subscribers: Mutex<HashMap< usize, Box<dyn Address<Wf, Error=PeerErr>> >>,
	id         : usize,
}



impl<Wf: WireFormat> PubSub<Wf>
{
	/// Create a new PubSub.
	/// The id parameter is the id of the address for this actor. From [`Identify::id()`].
	/// It's used for debugging logging so you can identify logged errors as coming from this actor.
	//
	pub fn new
	(
		services: Vec<ServiceID>,
		id: usize,
	)

		// TODO: Maybe this should not return a PeerErr, but a ThesErr, as this is no peer?
		//
		-> Result< Self, PeerErr >
	{
		Ok( Self
		{
			services,
			subscribers: Mutex::new( HashMap::new() ),
			id,
		})
	}


	pub fn subscribe( &mut self, sub: Box< dyn Address<Wf, Error=PeerErr> > )
	{
		let mut subs = self.subscribers.lock();

		subs.insert( sub.id(), sub );
	}


	pub fn unsubscribe( &mut self, id: usize )
	{
		let mut subs = self.subscribers.lock();

		subs.remove( &id );
	}
}



impl<Wf: WireFormat> ServiceMap<Wf> for PubSub<Wf>
{
	/// Send a message to a handler. This should take care of deserialization.
	//
	fn send_service( &self, msg: Wf, _ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	{
		trace!( "PubSub: Incoming Send for relayed subscribers." );

		let subs = self.subscribers.lock();

		// Run these concurrently so that we don't have one subscriber with a full queue blocking
		// the others from receiving.
		//
		let mut unordered: futures::stream::FuturesUnordered<_> = subs.values().map( |sub|
		{
			let mut sub = sub.clone_box();
			let     msg = msg.clone();
			let     id  = self.id;

			async move
			{
				if let Err(e) = sub.send( msg ).await
				{
					error!( "PubSub (id: {}): subscriber (id: {}, name: {:?}) fails to receive message: {}", id, sub.id(), sub.name(), e );
				}
			}

		}).collect();


		Ok( async move
		{
			while unordered.next().await.is_some() {}
			Ok( Response::Nothing )

		}.boxed())
	}


	/// PubSub implements a broadcast type fan out, so it doesn't support `Address::call`,
	/// as that requires a response. As we send to multiple receivers, which one is supposed to respond?
	//
	fn call_service( &self, _frame: Wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	{
		Err( PeerErr::PubSubNoCall{ ctx } )
	}


	fn services( &self ) -> Box<dyn Iterator<Item = &ServiceID> + '_ >
	{
		Box::new( self.services.iter() )
	}
}




impl<Wf> fmt::Debug for PubSub<Wf>
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "PubSub" )
	}
}



#[ derive( Debug ) ]
//
pub struct Subscribe<Wf = ThesWF>
{
	addr: Box<dyn Address<Wf, Error=PeerErr>>,
}

impl<Wf: 'static> Message for Subscribe<Wf> { type Return = (); }


impl<Wf: WireFormat> Handler<Subscribe<Wf>> for PubSub<Wf>
{
	#[async_fn] fn handle( &mut self, msg: Subscribe<Wf> ) -> <Subscribe<Wf> as Message>::Return
	{
		self.subscribe( msg.addr );
	}
}



#[ derive( Debug ) ]
//
pub struct UnSubscribe
{
	id: usize,
}

impl Message for UnSubscribe { type Return = (); }


impl<Wf: WireFormat> Handler<UnSubscribe> for PubSub<Wf>
{
	#[async_fn] fn handle( &mut self, msg: UnSubscribe ) -> <UnSubscribe as Message>::Return
	{
		self.unsubscribe( msg.id );
	}
}
