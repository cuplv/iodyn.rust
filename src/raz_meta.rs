//! Available meta data for the inc_gauged_raz
//! 
//! This meta data will be used when focusing into the Raz

use std::fmt::Debug;
use std::hash::Hash;
use adapton::engine::Name;

pub trait RazMeta<E>: Sized+Debug+Clone+Eq+Hash {
	type Index: FirstLast;

	/// create meta data for an empty branch
	fn from_none() -> Self;
	/// create meta data from a leaf vec
	fn from_vec(vec: &Vec<E>) -> Self;
	/// create meta data from the pair of meta data in branches
	///
	/// The names passed into here are USED in the tree that it
	/// rebuilds the meta data for, and should not be used again to
	/// name arts
	fn from_meta(l: &Self, r: &Self, lev: u32, n: Option<Name>) -> Self;
	/// choose a branch and create an adjusted index for that branch
	fn choose_side(l: &Self, r: &Self, index: &Self::Index) -> SideChoice<Self::Index>;
	/// splits a vec into slices based on the index
	fn split_vec<'a>(vec: &'a Vec<E>, index: &Self::Index) -> (&'a [E],&'a [E]);
}

/// A location and possibly an index for that location
pub enum SideChoice<T> {
	Left(T), Right(T), Here, Nowhere
}

pub trait FirstLast {
	fn first() -> Self;
	fn last() -> Self;
}

impl<E> RazMeta<E> for () {
	type Index = OnlyEnds;

	fn from_none() -> Self { () }
	fn from_vec(_vec: &Vec<E>) -> Self { () }
	fn from_meta(_l: &Self, _r: &Self, _lev: u32, _n: Option<Name>) -> Self { () }
	fn choose_side(_l: &Self, _r: &Self, index: &Self::Index) -> SideChoice<Self::Index> {
		match *index {
			OnlyEnds::First => SideChoice::Left(OnlyEnds::First),
			OnlyEnds::Last => SideChoice::Right(OnlyEnds::Last),
		}
	}
	fn split_vec<'a>(vec: &'a Vec<E>, index: &Self::Index) -> (&'a [E],&'a [E]) {
		match *index {
			OnlyEnds::First => vec.split_at(0),
			OnlyEnds::Last => vec.split_at(vec.len()),
		}
	}
}

#[derive(Debug,Clone,Eq,PartialEq,Hash)]
pub enum OnlyEnds {
	First,
	Last,
}

impl FirstLast for OnlyEnds {
	fn first() -> Self {OnlyEnds::First}
	fn last() -> Self {OnlyEnds::Last}
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

	fn from_none() -> Self { Count(0) }
	fn from_vec(vec: &Vec<E>) -> Self {
		Count(vec.len())
	}
	fn from_meta(&Count(l): &Self, &Count(r): &Self, _l: u32, _n: Option<Name>) -> Self {
		Count(l+r)
	}
	fn choose_side(
		&Count(l): &Self,
		&Count(r): &Self,
		index: &Self::Index
	) -> SideChoice<Self::Index> {
		let i = *index;
		if i == usize::max_value() { SideChoice::Right(i) }
		else if i > l + r { SideChoice::Nowhere }
		else if i > l { SideChoice::Right(i - l) }
		else if i == l { SideChoice::Here }
		else { SideChoice::Left(i) }
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

// TODO: HashMap