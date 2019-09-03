use crate::{ import::*, User };



#[ derive( Clone, Debug ) ]
//
pub struct Clients
{
	map: RwLock<HashMap<usize, User>>
}



impl Clients
{
	pub fn new() -> Self
	{
		Self { map: RwLock::new( HashMap::new() ) }
	}



	// If the map did not contain this user, None is return, otherwise the old value is returned
	//
	pub fn insert( &self, user: User ) -> Option<User>
	{
		self.map.write().insert( user.sid, user )
	}



	// Send a server message to all connected clients
	//
	pub async fn broadcast( &self, msg: &ServerMsg )
	{
		debug!( "start broadcast" );

		for user in self.map.read().values()
		{
			user.send( msg.clone() ).await;
		};

		debug!( "finish broadcast" );
	}



	pub fn users( &self ) -> Vec<(usize, String)>
	{
		self.map.read().values().map( |c| (c.sid, c.nick.read().clone()) ).collect()
	}


	// Send a server message to all connected clients
	//
	pub async fn send( &self, sid: usize, msg: ServerMsg ) -> Result<(), ()>
	{
		match self.map.read().get( &sid )
		{
			Some( user ) => user.send( msg.clone() ).await,
			None         => Err(())?,
		};

		Ok(())
	}


	// Send a server message to all connected clients
	//
	pub fn validate_nick( &self, sid: usize, old: Option<&str>, new: &str ) -> Result<(), ServerMsg>
	{
		// Check whether it's unchanged
		//
		if let Some(o) = old {
		if o == new
		{
			return Err( ServerMsg::NickUnchanged{ time: Utc::now().timestamp(), sid, nick: o.to_string() } );
		}}


		// Check whether it's not already in use
		//
		if self.map.read().values().any( |c| *c.nick.read() == new )
		{
			return Err( ServerMsg::NickInUse{ time: Utc::now().timestamp(), sid, nick: new.to_string() } )
		}


		// Check whether it's valid
		//
		let nickre   = Regex::new( r"^\w{1,15}$" ).unwrap();

		if !nickre.is_match( new )
		{
			error!( "Wrong nick: '{}'", new );
			return Err( ServerMsg::NickInvalid{ time: Utc::now().timestamp(), sid, nick: new.to_string() } )
		}


		// It's valid
		//
		// Ok( ServerMsg::NickChanged{ time: Utc::now().timestamp(), old: old.to_string(), new: new.to_string(), sid } )
		Ok(())
	}
}



