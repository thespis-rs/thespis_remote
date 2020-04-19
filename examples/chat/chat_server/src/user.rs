use crate::{ import::*, Server, UserData, ChangeNick, ChatMsgIn };

#[ derive( Actor ) ]
//
pub struct User
{
	server   : Addr<Server>                                            ,
	addr     : Option< Addr<Self>                                    > ,
	peer_addr: Option< Box< dyn Recipient<ServerMsg> + Send + Sync > > ,
	id       : usize                                                   ,
}



impl User
{
	pub fn new( server: Addr<Server>, addr: Addr<Self>, peer_addr: Box< dyn Recipient<ServerMsg> + Send + Sync > ) -> Self
	{
		User
		{
			server                       ,
			id       : addr.id()         ,
			addr     : Some( addr      ) ,
			peer_addr: Some( peer_addr ) ,
		}
	}
}



impl Handler< Join > for User
{
	fn handle( &mut self, msg: Join ) -> Return< Result<Welcome, ChatErr> > { Box::pin( async move
	{
		debug!( "received Join" );

		// You can only join once
		//
		if self.addr.is_none() || self.peer_addr.is_none()
		{
			Err( ChatErrKind::AlreadyJoined.into() )
		}

		// It just depends on whether the nick is valid
		//
		else
		{
			let new_user = UserData
			{
				addr: self.addr.take().unwrap(),
				nick: msg.nick,
				peer_addr: self.peer_addr.take().unwrap()
			};

			self.server.call( new_user ).await.expect( "call server" )
		}
	})}
}



impl Handler< SetNick > for User
{
	fn handle( &mut self, set_nick: SetNick ) -> Return< Result<(), ChatErr> > { Box::pin( async move
	{
		let change_nick = ChangeNick { sid: self.id, nick: set_nick.nick } ;

		self.server.call( change_nick ).await.expect( "call server" )

	})}
}



impl Handler< NewChatMsg > for User
{
	fn handle( &mut self, msg: NewChatMsg ) -> Return<()> { Box::pin( async move
	{
		let smsg = ChatMsgIn { sid: self.id, txt: msg.0 };

		self.server.send( smsg ).await.expect( "send to server" );

	})}
}

