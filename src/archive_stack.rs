//! Incremental High-gauge Archive Stack
//!
//! This implementation is incremental, meaning it
//! is articulated for use with Adapton. The user provides
//! names with each archive operation that are then used 
//! to memoize operations over the archived data.
//!
//! - a stack implemented with both persistent and mutable components
//! - insert in low-const O(1) - rare reallocations
//! - copy in high-const O(1) - short array copy then pointer
//!
//! The archive stack is implemented as a persistent stack of vectors.
//! Use `archive()` to save the current vector as the first entry in
//! the persistent linked list.

use std::mem;
use std::fmt::Debug;
use std::hash::Hash;
use stack::Stack;
use adapton::engine::Name;

/// Incremental Archive Stack
///
/// Operations on a mutable vector, with a persistent stack of 
/// vectors available. `pop()` will open archives, but pushing to
/// the archive requires the `archive()` method.
///
/// Parametric over elements, `E`, and metadata `M`, that will be
/// included with archived vectors. Metadata is currently only
/// available if the archive is accessed with [`next_archive()`]
///
/// [`next_archive()`]: struct.AStack.html#method.next_archive

#[derive(Debug,PartialEq,Eq,Clone,Hash)]
pub struct AStack<E:'static+Debug+Clone+Eq+Hash,M:'static+Debug+Clone+Eq+Hash> {
	// OPTIMIZE: Keeping separate vecs requires copying data when archiving
	// eventually we need to refactor to avoid this
	current: Vec<E>,
	archived: Stack<(M,Vec<E>)>,
}

/// Marker type for interpreting the stack as a sequence.
/// 
/// Assume the head of the sequence is the edit point.
/// Rust's default Vec has the edit point at the tail of the data.
#[derive(Clone)]
pub struct AtHead<T: 'static+Debug+Clone+Eq+Hash,M:'static+Debug+Clone+Eq+Hash>(
	pub AStack<T,M>
);
/// Marker type for interpreting the stack as a sequence.
/// 
/// Assume the tail of the sequence is the edit point.
/// Rust's default Vec has the edit point at the tail of the data.
#[derive(Clone)]
pub struct AtTail<T: 'static+Debug+Clone+Eq+Hash,M:'static+Debug+Clone+Eq+Hash>(
	pub AStack<T,M>
);

impl<E:'static+Debug+Clone+Eq+Hash, M:'static+Debug+Clone+Eq+Hash>
AStack<E,M> {
	/// new `AStack` with a new vector as current stack
	pub fn new() -> Self {
		AStack {
			current: Vec::new(),
			archived: Stack::new(),
		}
	}

	/// new `AStack` with a new pre-allocated vector as current stack
	pub fn with_capacity(capacity: usize) -> Self {
		AStack {
			current: Vec::with_capacity(capacity),
			archived: Stack::new(),
		}
	}

	/// whether or not the `AStack` has any data, including archived data
	pub fn is_empty(&self) -> bool { self.current.len() == 0  && self.archived.is_empty()}
	/// the total item count of the `AStack`, including archived data
	pub fn active_len(&self) -> usize { self.current.len() }
	/// get the incremental name of the archive, if it exists
	pub fn name(&self) -> Option<Name> { self.archived.name() }
	/// push a element to the "fast" vector outside of the archive
	pub fn push(&mut self, elm: E) { self.current.push(elm) }

	/// remove and return the element at the top of the stack, even if it is
	/// within the archive. 
	///
	/// archive metadata is lost when opening an archive in this way
	pub fn pop(&mut self) -> Option<E> {
		if self.is_empty() { return None }
		self.retrieve();
		self.current.pop()
	}

	/// remove and return the element at the top of the stack, even if it is
	/// withing the archive. If it was within the archive, return the metadata
	/// associated with that archive.
	pub fn pop_meta(&mut self) -> Option<(E,Option<M>)> {
		if self.is_empty() { return None }
		let meta = self.retrieve();
		self.current.pop().map(|e|{(e,meta)})
	}

	// extend the backing vector
	#[doc(hidden)]
	pub fn extend(&mut self, extra: &[E]) {
		self.current.extend_from_slice(extra);
	}

	// specialized use that will be removed in future updates
	#[doc(hidden)]
	pub fn extend_rev(&mut self, extra: &[E]) {
		self.current.extend_from_slice(extra);
		self.current.reverse();
	}

	/// Exposes the last data archived, returning the
	/// prior active vector and the metadata stored with the archive
	pub fn next_archive(&mut self) -> Option<(Vec<E>,Option<M>)> {
		if self.is_empty() { return None }
		if self.archived.is_empty() {
			let old_vec = mem::replace(&mut self.current, Vec::new());
			self.archived = Stack::new();
			return Some((old_vec,None));
		} else {
			let (old_meta,vec) = self.archived.peek().expect("missing data");
			let old_vec = mem::replace(&mut self.current, vec);
			self.archived = self.archived.pull().unwrap();
			return Some((old_vec, Some(old_meta)));
		}
	}

	/// Exposes the data archive and associated meta
	/// data, disregaurding the first metadata if active vector is empty
	pub fn next_archive_force(&mut self) -> Option<(Vec<E>,Option<M>)> {
		if let Some((v,m)) = self.next_archive() {
			if v.len() > 0 { Some((v,m)) } else { self.next_archive() }
		} else { None }
	}

	/// peeks at the entire active vector
	pub fn active_data(&self) -> &Vec<E> {
		&self.current
	}

	/// peeks at the element at the top of the stack, even if it
	/// is within the archive.
	///
	/// peeking into the archive does not open it, so it will
	/// retain any associated metadata
	pub fn peek(&self) -> Option<E> {
		if self.is_empty() { return None }
		if self.current.len() == 0 {
			let (_,v) = self.archived.peek().unwrap();
			Some(v.last().unwrap().clone())
		} else { Some(self.current.last().unwrap().clone()) }
	}

	pub fn peek_meta(&self) -> Option<M> {
		if self.is_empty() { return None }
		match self.archived.peek() {
			None => None,
			Some((m,_)) => Some(m),
		}
	}

	/// push the entire active vector into the archive, along with
	/// associated metadata
	/// return false if the active vector was empty. In this
	/// case, no archive will happen and the metadata will be unused
	pub fn archive(&mut self, name: Option<Name>, meta: M) -> bool {
		self.archive_with_capacity(name,meta, 0)
	}
	/// push the entire active vector into the archive, providing
	/// a capacity for the new active vector
	pub fn archive_with_capacity(&mut self, name: Option<Name>, meta: M, capacity: usize) -> bool {
		if self.current.len() == 0 { return false; }
		let old_vec = mem::replace(&mut self.current, Vec::with_capacity(capacity));
		self.archived = self.archived.push(name,(meta, old_vec));
		true
	}
	
	/// pulls the next vector from the archive if nessecary, returning
	/// archived metadata. Panics if there are no elements left
	fn retrieve(&mut self) -> Option<M> {
		if self.current.len() == 0 {
			let (meta,v) = self.archived.peek().unwrap();
	    self.current = v;
	    self.archived = self.archived.pull().unwrap();
	    Some(meta)
		} else { None }
	}
}

impl<E:'static+Debug+Clone+Eq+Hash, M:'static+Debug+Clone+Eq+Hash>
IntoIterator for AStack<E,M> {
	type Item = E;
	type IntoIter = Iter<E,M>;
	fn into_iter(self) -> Self::IntoIter {
		Iter(self)
	}
}

/// Iterator for elements of a archive stack
pub struct Iter<E:'static+Debug+Clone+Eq+Hash, M:'static+Debug+Clone+Eq+Hash>(AStack<E,M>);
impl<E:'static+Debug+Clone+Eq+Hash, M:'static+Debug+Clone+Eq+Hash>
Iterator for Iter<E,M> {
	type Item = E;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop()
	}
}

/// standard conversion, but no metadata will be included
impl<E:'static+Debug+Clone+Eq+Hash>
From<Vec<E>> for AStack<E,()> {
	fn from(v: Vec<E>) -> Self {
		AStack {
			current: v,
			archived: Stack::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use adapton::engine::*;

  #[test]
  fn test_retrieve() {
  	let mut stack = AStack::new();
  	stack.push(5);
  	stack.push(2);
  	stack.push(3);
  	let (nums,_) = stack.next_archive().unwrap();
  	assert_eq!(nums, vec!(5,2,3));

  	stack.push(4);
  	stack.push(2);
  	stack.push(4);
  	stack.archive(Some(name_of_usize(1)),());
  	stack.push(9);
  	stack.push(3);
  	stack.push(7);
  	stack.archive(Some(name_of_usize(2)),());

  	let (nums,_) = stack.next_archive().unwrap();
  	assert_eq!(nums, vec!());
  	let (nums,_) = stack.next_archive().unwrap();
  	assert_eq!(nums, vec!(9,3,7));
  	let (nums,_) = stack.next_archive().unwrap();
  	assert_eq!(nums, vec!(4,2,4));
  }

  #[test]
  fn test_through_archive() {
  	let mut stack = AStack::new();
  	stack.push(4);
  	stack.push(2);
  	stack.push(4);
  	stack.archive(Some(name_of_usize(1)),());
  	stack.push(9);
  	stack.push(3);

  	assert_eq!(Some(3), stack.pop());
  	assert_eq!(Some(9), stack.pop());
  	assert_eq!(Some(4), stack.pop());
  	assert_eq!(Some(2), stack.pop());
  }

  #[test]
  fn test_peek_archive() {
  	let mut stack = AStack::new();
  	stack.push(4);
  	stack.push(2);
  	stack.push(4);
  	stack.archive(Some(name_of_usize(1)),());
  	stack.push(9);
  	stack.push(3);

  	assert_eq!(Some(3), stack.peek());
  	assert_eq!(Some(3), stack.peek());
  	stack.pop();
  	assert_eq!(Some(9), stack.peek());
  	stack.pop();
  	assert_eq!(Some(4), stack.peek());
  	assert_eq!(Some(4), stack.peek());

  	stack.pop();
  	stack.pop();
  	stack.pop();
  	assert!(stack.is_empty());  	
  }

    #[test]
  fn test_iter() {
  	let mut stack = AStack::new();
  	stack.push(4);
  	stack.push(2);
  	stack.push(4);
  	stack.archive(Some(name_of_usize(1)),());
  	stack.push(9);
  	stack.push(3);
  	stack.push(7);
  	stack.archive(Some(name_of_usize(2)),());
  	stack.push(6);
  	stack.push(1);
  	stack.push(3);

  	let as_vec = stack.into_iter().collect::<Vec<_>>();
  	assert_eq!(vec![3,1,6,7,3,9,4,2,4], as_vec);
  }

}
