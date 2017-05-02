//! Incremental Linked List (Cons-list)
//! 
//! This list is augmented with Adapton articulations
//! to be of use in memoized functions. As development
//! continues, it will also be enhanced with internal
//! memoization of common list operations.
//! 
//! There are two data types, `Stack` for optionally empty 
//! lists and `Head` for lists with at least one element.
//! Conversion is through the public type `Stack<T>(Option<Head<T>>)`
//! All element types must be compatible with Adapton,
//! meaning they must implement `T:'static+Debug+Clone+Eq+Hash`.
//!
//! The API is still in development, one of these interfaces may
//! change to mimic the `Vec` interface.

use std::fmt::Debug;
use std::hash::Hash;

use adapton::engine::*;

/// Common linked-list
#[derive(Debug,PartialEq,Eq,Clone,Hash)]
pub struct Stack<T:'static+Debug+Clone+Eq+Hash>(pub Option<Head<T>>);

/// Linked list with at least one element
#[derive(Debug,PartialEq,Eq,Clone,Hash)]
pub struct Head<T:'static+Debug+Clone+Eq+Hash> {
	name: Option<Name>,
	main: Art<Body<T>>,
}
#[derive(Debug,PartialEq,Eq,Clone,Hash)]
struct Body<T:'static+Debug+Clone+Eq+Hash> {
	elem: T,
	next: Option<Head<T>>,
}

impl<T:'static+Debug+Clone+Eq+Hash>
Stack<T> {

	/// this is identical to `Stack(None)`
	pub fn new() -> Self {
		Stack(None)
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	/// return a stack with the new item as head
	pub fn push(&self, name: Option<Name>, elem: T) -> Self {
		Stack(Some(push_onto(self.0.clone(), name, elem)))
	}

	/// return the top item, if there is one
	pub fn peek(&self) -> Option<T> {
		self.0.as_ref().map(|h|{h.peek()})
	}

	/// get the incremental name of this stack, if it's not empty
	pub fn name(&self) -> Option<Name> {
		match self.0 {
			None => None,
			Some(ref h) => h.name.clone()
		}
	}

	/// return the stack without the top item (this is sometimes called `tail`)
	pub fn pull(&self) -> Option<Self> {
		self.0.as_ref().map(|h|{Stack(h.pull())})
	}

	/// return an iterator over the elements from the top of the stack
	pub fn iter(&self) -> Iter<T> {
		Iter{ next: self.0.clone() }
	}

}

impl<T:'static+Debug+Clone+Eq+Hash>
Head<T> {

	pub fn new(name: Option<Name>, elem: T) -> Self {
		push_onto(None, name, elem)
	}

	/// return a list with the new element as head
	pub fn push(&self, name: Option<Name>, elem: T) -> Self {
		push_onto(Some(self.clone()), name, elem)
	}

	/// return the head element
	pub fn peek(&self) -> T {
		force(&self.main).elem
	}

	/// get the incremental name of this list
	pub fn name(&self) -> Option<Name> {
		self.name.clone()
	}

	/// return the list without the head element
	pub fn pull(&self) -> Option<Self> {
		force(&self.main).next
	}

	/// return an iterator over the elements of the list
	pub fn iter(&self) -> Iter<T> {
		Iter { next: Some(self.clone()) }
	}

}

fn push_onto<T>(tail: Option<Head<T>>, name: Option<Name>, elem: T) -> Head<T> where
	T:'static+Debug+Clone+Eq+Hash
{
	match name {
		None => Head{
			name: None,
			main: put(Body{
				elem: elem,
				next: tail,
			})
		},
		Some(nm) => Head{
			name: Some(nm.clone()),
			main: cell(nm, Body{
				elem: elem,
				next: tail,
			})
		},
	}
}

/// Iterator for list items
pub struct Iter<T:'static+Debug+Clone+Eq+Hash> {
	next: Option<Head<T>>,
}

impl<T:'static+Debug+Clone+Eq+Hash>
Iter<T> {
	/// get the name prior to the next element
	pub fn name(&self) -> Option<Name> {
		match self.next {
			None => None,
			Some(ref h) => h.name.clone(),
		}
	}
}

impl<T:'static+Debug+Clone+Eq+Hash>
Iterator for Iter<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.next.take().map(|head| {
			let Body{elem,next} = force(&head.main);
			self.next = next;
			elem
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

  #[test]
  fn test_push_peek() {
  	let a = Stack::new();
  	let b = a
  		.push(Some(name_of_usize(5)),5)
  		.push(Some(name_of_usize(6)),6);
  	assert_eq!(6, b.peek().unwrap());

  	let c = b.pull().unwrap();
  	assert_eq!(5, c.peek().unwrap());
  }

  #[test]
  fn test_iter() {
		let a = 
			Head::new(Some(name_of_usize(4)),4)
			.push(Some(name_of_usize(3)),3)
			.push(Some(name_of_usize(2)),2)
			.push(Some(name_of_usize(1)),1);
		assert_eq!(vec![1,2,3,4], a.iter().collect::<Vec<_>>());
	} 

}