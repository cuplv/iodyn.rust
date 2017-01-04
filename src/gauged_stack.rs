// High-gauge Stack
// - a stack implemented with both persistent and mutable components
// - insert in low-const O(1) - rare reallocations
// - copy in high-const O(1) - short array copy then pointer

use std::mem;
use stack::Stack;

pub struct GStack<E: Clone,M: Clone> {
	size: usize,
	meta: M,
	grain: Vec<E>,
	grains: Stack<(M,Vec<E>)>,
}

impl<E: Clone, M: Clone> GStack<E,M> {
	pub fn new(meta: M) -> Self {
		GStack {
			size: 0,
			meta: meta,
			grain: Vec::new(),
			grains: Stack::new(),
		}
	}

	pub fn is_empty(&self) -> bool { self.size == 0 }
	pub fn len(&self) -> usize { self.size }
	pub fn push(&mut self, elm: E) { self.size += 1; self.grain.push(elm) }
	pub fn pop(&mut self) -> Option<E> {
		if self.size == 0 { return None }
		self.retrieve();
		self.size -= 1;
		self.grain.pop()
	}
	pub fn pop_vec(&mut self) -> Option<(M,Vec<E>)> {
		if self.size == 0 { return None }
		let lost_len = self.grain.len();
		if lost_len == self.size {
			self.size = 0;
			let old_grain = mem::replace(&mut self.grain, Vec::new());
			self.grains = Stack::new();
			return Some((self.meta.clone(), old_grain));
		} else {
			self.size -= lost_len;
			let (old_meta, old_grain);
			{
				let &(ref m,ref v) = self.grains.peek().expect("missing data");
				old_meta = mem::replace(&mut self.meta, m.clone());
				old_grain = mem::replace(&mut self.grain, v.clone());
			}
			self.grains = self.grains.pull().unwrap();
			return Some((old_meta, old_grain));
		}

	}
	pub fn peek(&mut self) -> Option<&E> {
		if self.size == 0 { return None }
		self.retrieve();
		self.grain.last()
	}
	pub fn archive(&mut self, new_meta: M) {
		// should we ensure that there are no empty vec's archived?

		// why won't the compiler alter this code for me automatically?
		// self.grains = self.grains.push((self.meta,self.grain));
		// self.grain = Vec::new();
		// self.meta = meta;
		let old_meta = mem::replace(&mut self.meta, new_meta);
		let old_grain = mem::replace(&mut self.grain, Vec::new());
		self.grains = self.grains.push((old_meta, old_grain));
	}
	
	// pulls the next non-empty grain from the grains archive, overwriting
	// current metadata. Panics if there are no elements left
	fn retrieve(&mut self) {
		while self.grain.len() == 0 {
			// should our metadata be saved somehow?
			{
				let &(ref m,ref v) = self.grains.peek().unwrap();
		    self.meta = m.clone();
		    self.grain = v.clone();
		  }
	    self.grains = self.grains.pull().unwrap()
		}
	}
}

impl<E: Clone, M:Clone> Clone for GStack<E,M> {
	fn clone(&self) -> Self {
		GStack {
			size: self.size,
			meta: self.meta.clone(),
			grain: self.grain.clone(),
			grains: self.grains.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

  #[test]
  fn test_retrieve() {
  	let mut stack = GStack::new(());
  	stack.push(5);
  	stack.push(2);
  	stack.push(3);
  	let (_,nums) = stack.pop_vec().unwrap();
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

  	let (_,nums) = stack.pop_vec().unwrap();
  	assert_eq!(nums, vec!());
  	let (_,nums) = stack.pop_vec().unwrap();
  	assert_eq!(nums, vec!(9,3,7));
  	let (_,nums) = stack.pop_vec().unwrap();
  	assert_eq!(nums, vec!(4,2,4));
  }

  #[test]
  fn test_pop_archive() {
  	let mut stack = GStack::new(());
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
  	let mut stack = GStack::new(());
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
