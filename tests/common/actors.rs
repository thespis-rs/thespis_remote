use super::import::*;

use serde:: { Serialize, Deserialize };



#[ derive( Actor ) ] pub struct Sum( pub i64 );

#[ derive( Serialize, Deserialize, Debug ) ] pub struct Add( pub i64 );
#[ derive( Serialize, Deserialize, Debug ) ] pub struct Sub( pub i64 );
#[ derive( Serialize, Deserialize, Debug ) ] pub struct Show;

impl Message for Add  { type Return = ();  }
impl Message for Sub  { type Return = ();  }
impl Message for Show { type Return = i64; }



impl Handler< Add > for Sum
{
	#[async_fn] fn handle( &mut self, msg: Add ) -> ()
	{
		trace!( "called sum with: {:?}", msg );

		self.0 += msg.0;

	}
}



impl Handler< Sub > for Sum
{
	#[async_fn] fn handle( &mut self, msg: Sub ) -> ()
	{
		trace!( "called sum with: {:?}", msg );

		self.0 -= msg.0;
	}
}



impl Handler< Show > for Sum
{
	#[async_fn] fn handle( &mut self, _msg: Show ) -> i64
	{
		trace!( "called sum with: Show" );

		self.0
	}
}
