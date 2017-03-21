use std::fmt::Debug;
use std::hash::Hash;

use adapton::engine::*;

#[derive(Debug,PartialEq,Eq,Clone,Hash)]
pub struct Stack<T:'static+Debug+Clone+Eq+Hash>(pub Option<Head<T>>);

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
	pub fn new() -> Self {
		Stack(None)
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	pub fn push(self, name: Option<Name>, elem: T) -> Self {
		Stack(Some(push_onto(self.0, name, elem)))
	}

	pub fn peek(&self) -> Option<T> {
		self.0.as_ref().map(|s|{s.peek()})
	}

	pub fn pull(&self) -> Option<Self> {
		self.0.as_ref().map(|s|{Stack(s.pull())})
	}

	pub fn iter(&self) -> Iter<T> {
		Iter{ next: self.0.clone() }
	}

}

impl<T:'static+Debug+Clone+Eq+Hash>
Head<T> {

	pub fn new(name: Option<Name>, elem: T) -> Self {
		push_onto(None, name, elem)
	}

	pub fn push(self, name: Option<Name>, elem: T) -> Self {
		push_onto(Some(self), name, elem)
	}

	pub fn peek(&self) -> T {
		force(&self.main).elem
	}

	pub fn pull(&self) -> Option<Self> {
		force(&self.main).next
	}

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