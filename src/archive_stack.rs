//! High-gauge Stack
//!
//! - a stack implemented with both persistent and mutable components
//! - insert in low-const O(1) - rare reallocations
//! - copy in high-const O(1) - short array copy then pointer
//!
//! The archive stack is implemented as a persistent stack of vectors.
//! Use `archive()` to save the current vector as the first entry in
//! the persistent linked list.

use std::mem;
use stack::Stack;

/// Archive Stack
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
pub struct AStack<E: Clone,M: Clone> {
	size: usize,
	current: Vec<E>,
	archived: Stack<(M,Vec<E>)>,
}

impl<E: Clone, M: Clone> AStack<E,M> {
	/// new `AStack` with a new vector as current stack
	pub fn new() -> Self {
		AStack {
			size: 0,
			current: Vec::new(),
			archived: Stack::new(),
		}
	}

	/// new `AStack` with a new pre-allocated vector as current stack
	pub fn with_capacity(capacity: usize) -> Self {
		AStack {
			size: 0,
			current: Vec::with_capacity(capacity),
			archived: Stack::new(),
		}
	}

	/// whether or not the `AStack` has any data, including archived data
	pub fn is_empty(&self) -> bool { self.size == 0 }
	/// the total item count of the `AStack`, including archived data
	pub fn len(&self) -> usize { self.size }
	/// the item count of the "fast" mutable vector outside of the archive
	pub fn active_len(&self) -> usize { self.current.len() }
	/// push a element to the "fast" vector outside of the archive
	pub fn push(&mut self, elm: E) { self.size += 1; self.current.push(elm) }
	/// remove and return the element at the top of the stack, even if it is
	/// within the archive. 
	///
	/// archive metadata is lost when opening an archive in this way
	pub fn pop(&mut self) -> Option<E> {
		if self.size == 0 { return None }
		self.retrieve();
		self.size -= 1;
		self.current.pop()
	}

	// extend the backing vector
	#[doc(hidden)]
	pub fn extend(&mut self, extra: &[E]) {
		self.size += extra.len();
		self.current.extend_from_slice(extra);
	}

	// specialized use that will be removed in future updates
	#[doc(hidden)]
	pub fn extend_rev(&mut self, extra: &[E]) {
		self.size += extra.len();
		self.current.extend_from_slice(extra);
		self.current.reverse();
	}

	/// Exposes the last data archived, returning the
	/// prior active vector and the metadata stored with the archive
	pub fn next_archive(&mut self) -> Option<(Vec<E>,Option<M>)> {
		if self.size == 0 { return None }
		let lost_len = self.current.len();
		if lost_len == self.size {
			self.size = 0;
			let old_vec = mem::replace(&mut self.current, Vec::with_capacity(500));
			self.archived = Stack::new();
			return Some((old_vec,None));
		} else {
			self.size -= lost_len;
			let (old_meta, old_vec);
			{
				let &(ref m,ref v) = self.archived.peek().expect("missing data");
				old_meta = m.clone();
				old_vec = mem::replace(&mut self.current, v.clone());
			}
			self.archived = self.archived.pull().unwrap();
			return Some((old_vec, Some(old_meta)));
		}
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
	pub fn peek(&self) -> Option<&E> {
		if self.size == 0 { return None }
		if self.current.len() == 0 {
			let &(_,ref v) = self.archived.peek().unwrap();
	    v.last()
		} else { self.current.last() }
	}
	/// push the entire active vector into the archive, along with
	/// associated metadata
	/// return false if the active vector was empty. In this
	/// case, no archive will happen and the metadata will be unused
	pub fn archive(&mut self, new_meta: M) -> bool {
		if self.current.len() == 0 { return false; }
		let old_vec = mem::replace(&mut self.current, Vec::new());
		self.archived = self.archived.push((new_meta, old_vec));
		true
	}
	/// push the entire active vector into the archive, providing
	/// a capacity for the new active vector
	pub fn archive_with_capacity(&mut self, new_meta: M, capacity: usize) -> bool {
		if self.current.len() == 0 { return false; }
		let old_vec = mem::replace(&mut self.current, Vec::with_capacity(capacity));
		self.archived = self.archived.push((new_meta, old_vec));
		true
	}
	
	/// pulls the next vector from the archive if nessecary, destroying
	/// archived metadata. Panics if there are no elements left
	fn retrieve(&mut self) {
		if self.current.len() == 0 {
			{
				let &(_,ref v) = self.archived.peek().unwrap();
		    self.current = v.clone();
		  }
	    self.archived = self.archived.pull().unwrap();
		}
	}
}

impl<E: Clone,M: Clone> IntoIterator for AStack<E,M> {
	type Item = E;
	type IntoIter = Iter<E,M>;
	fn into_iter(self) -> Self::IntoIter {
		Iter(self)
	}
}

pub struct Iter<E: Clone,M: Clone>(AStack<E,M>);
impl<E: Clone,M: Clone> Iterator for Iter<E,M> {
	type Item = E;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop()
	}
}

/// standard conversion, but no metadata will be included
impl<E: Clone> From<Vec<E>> for AStack<E,()> {
	fn from(v: Vec<E>) -> Self {
		AStack {
			size: v.len(),
			current: v,
			archived: Stack::new(),
		}
	}
}

impl<E: Clone, M:Clone> Clone for AStack<E,M> {
	fn clone(&self) -> Self {
		AStack {
			size: self.size,
			current: self.current.clone(),
			archived: self.archived.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
  	stack.archive(());
  	stack.push(9);
  	stack.push(3);
  	stack.push(7);
  	stack.archive(());

  	assert_eq!(stack.len(), 6);

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
  	stack.archive(());
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
  	stack.archive(());
  	stack.push(9);
  	stack.push(3);

  	assert_eq!(Some(&3), stack.peek());
  	assert_eq!(Some(&3), stack.peek());
  	stack.pop();
  	assert_eq!(Some(&9), stack.peek());
  	stack.pop();
  	assert_eq!(Some(&4), stack.peek());
  	assert_eq!(Some(&4), stack.peek());

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
  	stack.archive(());
  	stack.push(9);
  	stack.push(3);
  	stack.push(7);
  	stack.archive(());
  	stack.push(6);
  	stack.push(1);
  	stack.push(3);

  	let as_vec = stack.into_iter().collect::<Vec<_>>();
  	assert_eq!(vec![3,1,6,7,3,9,4,2,4], as_vec);
  }

}
