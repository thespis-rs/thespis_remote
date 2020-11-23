use crate::{ *, import::* };

type Subscriber<Wf> = Box<dyn Address<Wf, Error=ThesErr>>;

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
pub struct PubSub<Wf: 'static = ThesWF>
{
	services   : Vec<ServiceID>,

	// We use this mutex in an async task, but it should be fine. It's never
	// held accross await points, and it's only ever held for short periods of time.
	//
	subscribers: Arc<Mutex<HashMap< usize, Subscriber<Wf> >>>,
	rt_sub     : Option< JoinHandle<()> >,
	rt_unsub   : Option< JoinHandle<()> >,
}



impl<Wf: WireFormat> PubSub<Wf>
{
	/// Create a new PubSub.
	/// The id parameter is the id of the address for this actor. From [`Identify::id()`].
	/// It's used for debugging logging so you can identify logged errors as coming from this actor.
	//
	pub fn new( services: Vec<ServiceID> ) -> Self
	{
		Self
		{
			services                                            ,
			subscribers: Arc::new(Mutex::new( HashMap::new() )) ,
			rt_sub     : None                                   ,
			rt_unsub   : None                                   ,
		}
	}


	/// Subscribe a relay before handing off this PubSub to Peer. If you want to be able
	/// to add new subscribers during runtime, use [`PubSub::rt_subscribe`].
	//
	pub fn subscribe( &mut self, sub: Box< dyn Address<Wf, Error=ThesErr> > )
	{
		let mut subs = self.subscribers.lock();

		subs.insert( sub.id(), sub );
	}


	/// Unsubscribe a relay before handing off this PubSub to Peer. If you want to be able
	/// to remove subscribers during runtime, use [`PubSub::rt_unsubscribe`].
	//
	pub fn unsubscribe( &mut self, id: usize )
	{
		let mut subs = self.subscribers.lock();

		subs.remove( &id );
	}


	/// Take a channel Sender that you can send new subscribers to at any time.
	/// This will spawn a task that listens to this channel to update our subscribers
	/// list as needed. When this `PubSub` get's dropped or you drop the sender,
	/// that task will be canceled and dropped.
	//
	pub fn rt_subscribe( &mut self, exec: &impl SpawnHandle<()> )

		-> Result< futUnboundSender< Subscriber<Wf> >, ThesErr >
	{
		assert!( self.rt_sub.is_none(), "Can only call rt_subscribe once. Clone the Sender if you need it in several places." );

		let (tx, mut rx) = mpsc::unbounded::<Subscriber<Wf>>();
		let map          = self.subscribers.clone();

		let task = async move
		{
			while let Some(sub) = rx.next().await
			{
				let mut m = map.lock();

				m.insert( sub.id(), sub );
			}
		};

		self.rt_sub = Some
		(
			exec.spawn_handle(task)
			.map_err( |_| ThesErr::Spawn{ actor: "PubSub".to_string() } )?
		);

		Ok(tx)
	}


	/// Take a channel Sender that you can send subscriber id's to to unsubscribe to at any time.
	/// This will spawn a task that listens to this channel to update our subscribers
	/// list as needed. When this `PubSub` get's dropped or you drop the sender,
	/// that task will be canceled and dropped.
	//
	pub fn rt_unsubscribe( &mut self, exec: &impl SpawnHandle<()> )

		-> Result< futUnboundSender< usize >, ThesErr >
	{
		assert!( self.rt_unsub.is_none(), "Can only call rt_subscribe once. Clone the Sender if you need it in several places." );

		let (tx, mut rx) = mpsc::unbounded::<usize>();
		let map          = self.subscribers.clone();

		let task = async move
		{
			while let Some(sub) = rx.next().await
			{
				let mut m = map.lock();

				m.remove( &sub );
			}
		};

		self.rt_unsub = Some
		(
			exec.spawn_handle(task)
			.map_err( |_| ThesErr::Spawn{ actor: "PubSub".to_string() } )?
		);

		Ok(tx)
	}
}



impl<Wf: WireFormat> ServiceMap<Wf> for PubSub<Wf>
{
	/// Send a message to a handler. This should take care of deserialization.
	//
	fn send_service( &self, msg: Wf, ctx: PeerErrCtx )

		-> Result< Pin<Box< dyn Future< Output=Result<Response<Wf>, PeerErr> > + Send >>, PeerErr >
	{
		trace!( "PubSub: Incoming Send for relayed subscribers." );

		let peer_id = ctx.peer_id;

		let mut unordered: FuturesUnordered<_> =
		{
			// Run these concurrently so that we don't have one subscriber with a full queue blocking
			// the others from receiving.
			//
			self.subscribers.lock().values().map( |sub|
			{
				let mut sub = sub.clone_box();
				let     msg = msg.clone();

				async move
				{
					debug!( "PubSub: Send to subscriber" );

					if let Err(e) = sub.send( msg ).await
					{
						error!
						(
							"PubSub (Peer id: {:?}): subscriber (id: {}, name: {:?}) fails to receive message: {}",
							peer_id, sub.id(), sub.name(), e
						);
					}
				}

			}).collect()
		};


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
