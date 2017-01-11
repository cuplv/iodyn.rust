// High-gauge Stack
// - a stack implemented with both persistent and mutable components
// - insert in low-const O(1) - rare reallocations
// - copy in high-const O(1) - short array copy then pointer

use std::mem;
use stack::Stack;

pub struct AStack<E: Clone,M: Clone> {
	size: usize,
	current: Vec<E>,
	archived: Stack<(M,Vec<E>)>,
}

impl<E: Clone, M: Clone> AStack<E,M> {
	pub fn new() -> Self {
		AStack {
			size: 0,
			current: Vec::new(),
			archived: Stack::new(),
		}
	}

	pub fn is_empty(&self) -> bool { self.size == 0 }
	pub fn len(&self) -> usize { self.size }
	pub fn push(&mut self, elm: E) { self.size += 1; self.current.push(elm) }
	pub fn pop(&mut self) -> Option<E> {
		if self.size == 0 { return None }
		self.retrieve();
		self.size -= 1;
		self.current.pop()
	}

	pub fn extend(&mut self, extra: &[E]) {
		self.size += extra.len();
		self.current.extend_from_slice(extra);
	}

	// specialized use that will be removed in future updates
	pub fn extend_rev(&mut self, extra: &[E]) {
		self.size += extra.len();
		self.current.extend_from_slice(extra);
		self.current.reverse();
	}

	// moves on to the last data archived, returning the
	// prior active data and the metadata stored with the archive
	pub fn next_archive(&mut self) -> Option<(Vec<E>,Option<M>)> {
		if self.size == 0 { return None }
		let lost_len = self.current.len();
		if lost_len == self.size {
			self.size = 0;
			let old_vec = mem::replace(&mut self.current, Vec::new());
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

	pub fn active_data(&self) -> &Vec<E> {
		&self.current
	}

	pub fn peek(&self) -> Option<&E> {
		if self.size == 0 { return None }
		if self.current.len() == 0 {
			let &(_,ref v) = self.archived.peek().unwrap();
	    v.last()
		} else { self.current.last() }
	}
	pub fn archive(&mut self, new_meta: M) -> bool {
		if self.current.len() == 0 { return false; }
		let old_vec = mem::replace(&mut self.current, Vec::new());
		self.archived = self.archived.push((new_meta, old_vec));
		true
	}
	
	// pulls the next vec from archive, destroying
	// archived metadata. Panics if there are no elements left
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
}
