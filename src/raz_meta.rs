//! Available meta data for the inc_gauged_raz
//! 
//! This meta data will be used when focusing into the Raz

use std::fmt::Debug;
use std::hash::{Hash,Hasher};
use std::collections::HashMap;
use adapton::engine::Name;

/// trait for creating and searching for meta data in the
/// branches of the raz.
/// 
/// There are two pieces of meta data
/// in each node, one created from its left branch and one
/// created from its right branch. The level and name from
/// the current node are also available.
pub trait RazMeta<E>: Sized+Debug+Clone+Eq+Hash {
	type Index: FirstLast;

	/// create meta data for an empty branch
	///
	/// The names passed into here are USED in the tree that it
	/// rebuilds the meta data for, and should not be used again to
	/// name arts
	fn from_none(lev: u32, n: Option<Name>) -> Self;
	/// create meta data from a leaf vec
	///
	/// The names passed into here are USED in the tree that it
	/// rebuilds the meta data for, and should not be used again to
	/// name arts
	fn from_vec(vec: &Vec<E>, lev: u32, n: Option<Name>) -> Self;
	/// create meta data from the pair of meta data in branches
	///
	/// Each branch contains a pair of meta data, so if this fn
	/// is being used to create left meta data, it will receive
	/// the left and right pair from the left branch, along with
	/// the current level and name. This is similar for creating
	/// the right meta data.
	///
	/// The names passed into here are USED in the tree that it
	/// rebuilds the meta data for, and should not be used again to
	/// name arts
	fn from_meta(l: &Self, r: &Self, lev: u32, n: Option<Name>) -> Self;
	/// choose a branch and create an adjusted index for that branch
	fn navigate(l: &Self, r: &Self, index: &Self::Index) -> Navigation<Self::Index>;
	/// splits a vec into slices based on the index
	fn split_vec<'a>(vec: &'a Vec<E>, index: &Self::Index) -> (&'a [E],&'a [E]);
}

/// A location and possibly an index for that location
pub enum Navigation<I> {
	Left(I), Right(I), Here, Nowhere
}

/// Trait to set search position at either end of a sequence
pub trait FirstLast {
	fn first() -> Self;
	fn last() -> Self;
}

/// A common pattern for indexes
///
/// Exposes the common left and right
/// cases necessary for meta indexes, with
/// a center case that takes a more specific
/// index.
#[derive(Debug,Clone,Eq,PartialEq,Hash)]
pub enum Position<I> {
	Left, Right, Center(I)
}

impl<I> FirstLast for Position<I> {
	fn first() -> Self {Position::Left}
	fn last() -> Self {Position::Right}
}

impl<I> From<I> for Position<I> {
	fn from(index: I) -> Self { Position::Center(index) }
}

impl<E> RazMeta<E> for () {
	type Index = Position<()>;

	fn from_none(_lev: u32, _n: Option<Name>) -> Self { () }
	fn from_vec(_vec: &Vec<E>, _lev: u32, _n: Option<Name>) -> Self { () }
	fn from_meta(_l: &Self, _r: &Self, _lev: u32, _n: Option<Name>) -> Self { () }
	fn navigate(_l: &Self, _r: &Self, index: &Self::Index) -> Navigation<Self::Index> {
		match *index {
			Position::Left => Navigation::Left(Position::Left),
			Position::Right => Navigation::Right(Position::Right),
			_ => panic!("Invalid position"),
		}
	}
	fn split_vec<'a>(vec: &'a Vec<E>, index: &Self::Index) -> (&'a [E],&'a [E]) {
		match *index {
			Position::Left => vec.split_at(0),
			Position::Right => vec.split_at(vec.len()),
			_ => panic!("Invalid position"),
		}
	}
}

/// Meta data for element count and positioning from the left
///
/// usize::max_value() is a special marker for
/// the end of the sequence, otherwise, values too large will
/// fail in some appropriate way.
#[derive(Clone,Eq,PartialEq,Hash,Debug)]
pub struct Count(pub usize);

impl<E> RazMeta<E> for Count {
	type Index = usize;

	fn from_none(_lev: u32, _n: Option<Name>) -> Self { Count(0) }
	fn from_vec(vec: &Vec<E>, _lev: u32, _n: Option<Name>) -> Self {
		Count(vec.len())
	}
	fn from_meta(&Count(l): &Self, &Count(r): &Self, _l: u32, _n: Option<Name>) -> Self {
		Count(l+r)
	}
	fn navigate(
		&Count(l): &Self,
		&Count(r): &Self,
		index: &Self::Index
	) -> Navigation<Self::Index> {
		let i = *index;
		if i == usize::max_value() { Navigation::Right(i) }
		else if i > l + r { Navigation::Nowhere }
		else if i > l { Navigation::Right(i - l) }
		else if i == l { Navigation::Here }
		else { Navigation::Left(i) }
	}
	/// # Panics
	/// Panics if the index is too high
	fn split_vec<'a>(vec: &'a Vec<E>, index: &Self::Index) -> (&'a [E],&'a [E]) {
		if *index == usize::max_value() {
			vec.split_at(vec.len())
		} else {
			vec.split_at(*index)	
		}
	}	
}

impl FirstLast for usize {
	fn first() -> Self { 0 }
	fn last() -> Self { usize::max_value() }
}

/// Metadata for names in a raz tree.
///
/// Hash is implemented by a no-op, since the data here
/// can be found elsewhere in the tree. This is not intended
/// to be used outside of a raz.
#[derive(Clone,Eq,PartialEq,Debug)]
pub struct Names(pub HashMap<Name,()>);
impl Hash for Names {
	/// does nothing
	fn hash<H:Hasher>(&self, _state: &mut H) {}
}

impl<E> RazMeta<E> for Names {
	type Index = Position<Name>;

	fn from_none(_lev: u32, n: Option<Name>) -> Self {
		let mut h = HashMap::new();
		match n {None=>{},Some(nm)=>{ h.insert(nm,()); }}
		Names(h)
	}
	fn from_vec(_vec: &Vec<E>, _lev: u32, n: Option<Name>) -> Self {
		let mut h = HashMap::new();
		match n {None=>{},Some(nm)=>{ h.insert(nm,()); }}
		Names(h)
	}
	fn from_meta(l: &Self, r: &Self, _lev: u32, n: Option<Name>) -> Self {
		let mut h = HashMap::new();
		for k in l.0.keys() { h.insert(k.clone(),()); }
		for k in r.0.keys() { h.insert(k.clone(),()); }
		match n {None=>{},Some(nm)=>{ h.insert(nm,()); }}
		Names(h)
	}
	fn navigate(l: &Self, r: &Self, index: &Self::Index) -> Navigation<Self::Index> {
		match *index {
			Position::Left => Navigation::Left(Position::Left),
			Position::Right => Navigation::Right(Position::Right),
			Position::Center(ref nm) => {
				match (l.0.contains_key(nm),r.0.contains_key(nm)) {
					(true,true) => Navigation::Here,
					(true,false) => Navigation::Left(index.clone()),
					(false,true) => Navigation::Right(index.clone()),
					(false,false) => Navigation::Nowhere,
				}
			}
		}
	}
	/// # Panics
	/// Panics if a name is given as index 
	fn split_vec<'a>(vec: &'a Vec<E>, index: &Self::Index) -> (&'a [E],&'a [E]) {
		match *index {
			Position::Left => vec.split_at(0),
			Position::Right => vec.split_at(vec.len()),
			_ => panic!("There are no names in this region")
		}
	}
}