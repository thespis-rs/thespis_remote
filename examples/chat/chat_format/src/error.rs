use crate::{ import::* };


/// The error type for errors happening in `ws_stream`.
///
/// Use [`ChatErr::kind()`] to know which kind of error happened.
//
#[ derive( Debug, Serialize, Deserialize ) ]
//
pub struct ChatErr
{
	pub(crate) kind : ChatErrKind,
}



/// The different kind of errors that can happen when you use the `ws_stream` API.
//
#[ derive( Debug, Serialize, Deserialize ) ]
//
pub enum ChatErrKind
{
	/// The new nickname is invalid (must be 1-15 word characters (\w regex class)).
	//
	NickInvalid,

	/// An error happend on the tcp level when connecting.
	//
	NickInuse,

	/// The old nickname is the same as the new one.
	//
	NickUnchanged,

	/// Can only call join once.
	//
	AlreadyJoined,


	#[ doc( hidden ) ]
	//
	__NonExhaustive__
}



impl ErrorTrait for ChatErr {}




impl fmt::Display for ChatErrKind
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		match self
		{
			Self::NickInvalid   => fmt::Display::fmt( "The new nickname is invalid (must be 1-15 word characters (\\w regex class)).", f ) ,
			Self::NickInuse     => fmt::Display::fmt( "An error happend on the tcp level when connecting.", f ) ,
			Self::NickUnchanged => fmt::Display::fmt( "The old nickname is the same as the new one.", f ) ,
			Self::AlreadyJoined => fmt::Display::fmt( "You can only call join once.", f ) ,

			_ => unreachable!(),
		}
	}
}


impl fmt::Display for ChatErr
{
	fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
	{
		write!( f, "ChatErr: {}", self.kind )
	}
}



impl ChatErr
{
	/// Allows matching on the error kind
	//
	pub fn kind( &self ) -> &ChatErrKind
	{
		&self.kind
	}
}

impl From<ChatErrKind> for ChatErr
{
	fn from( kind: ChatErrKind ) -> ChatErr
	{
		ChatErr { kind }
	}
}
