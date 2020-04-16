use crate::import::*;
use std::{ borrow::Borrow, hash::Hash, collections::hash_map::Entry };

#[ derive( Debug, Clone ) ]
//
pub(crate) struct HashMany<K, V>

	where K: Eq + Hash
{
	keys  : HashMap<K, usize>  ,
	values: HashMap< usize, V> ,
	next  : usize              ,
}


impl<K, V> HashMany<K, V>

	where K: Eq + Hash

{
	pub(crate) fn new() -> Self
	{
		Self::default()
	}


	pub(crate) fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool

		where K: Borrow<Q>,
		      Q: Hash + Eq,

	{
		self.keys.contains_key( k )
	}


	pub(crate) fn clear( &mut self )
	{
		self.keys.clear();
		self.values.clear();
		self.next = 0;
	}


	pub(crate) fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>

		where K: Borrow<Q>,
		      Q: Hash + Eq,

	{
		self.keys.get( &k )

			.map( |idx| self.values.get( idx ) )
			.flatten()
	}


	pub(crate) fn insert( &mut self, k: K, v: V ) -> Option<V>
	{
		match self.keys.get( &k )
		{
			Some(idx) =>
			{
				match self.values.entry( *idx )
				{
					Entry::Occupied(mut o) =>
					{
						Some( o.insert( v ) )
					}

					_ => unreachable!(),
				}
			}

			None =>
			{
				self.values.insert( self.next, v );
				self.keys  .insert( k, self.next );

				self.next += 1;
				None
			}
		}
	}


	/// Insert a value with a list of keys. If any of the keys exist, they will now point
	/// to the new value. If any values become orphaned by this they will be dropped.
	//
	pub(crate) fn insert_many<T>( &mut self, keys: T, v: V )

		where T            : IntoIterator< Item=K  >,
		      for<'a> &'a T: IntoIterator< Item=&'a K > ,
		      K            : Clone,
	{
		let mut zero_size = true;
		// let keys = keys.into_iter();
		// let refkeys      = &keys;

		for k in &keys
		{
			self.remove( k );
			zero_size = false;
		}

		// Needed for the unwrap below.
		//
		if zero_size { return }

		let mut keys = keys.into_iter();
		let     k1   = keys.next().unwrap();

		// Insert the value.
		//
		self.insert( k1.clone(), v );

		for k in keys
		{
			self.alias( &k1, k );
		}
	}


	pub(crate) fn alias<Q: ?Sized>( &mut self, k: &Q, new: K ) -> Option<&V>

		where K: Borrow<Q>,
		      Q: Hash + Eq,

	{
		let idx = *self.keys.get( &k )?;

		self.keys.insert( new, idx );

		self.values.get( &idx )
	}


	/// Remove the key from the `HashMany`. If there are no other keys pointing to the same value, the
	/// value is removed and returned.
	///
	/// If `None` is returned, either the key didn't exist or it has aliases pointing to the same value.
	//
	pub(crate) fn remove<Q: ?Sized>( &mut self, k: &Q ) -> Option<V>

		where K: Borrow<Q>,
		      Q: Hash + Eq,

	{
		let idx = self.keys.remove( k )?;

		if self.keys.iter().find( |(_k, v)| **v == idx ).is_none()
		{
			self.values.remove( &idx )
		}

		else { None }

	}
}


impl<K, V> Default for HashMany<K, V>

	where K: Eq + Hash

{
	fn default() -> Self
	{
		Self
		{
			keys  : Default::default() ,
			values: Default::default() ,
			next  : Default::default() ,
		}
	}
}
