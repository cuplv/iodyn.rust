use std::fmt::Debug;
use std::hash::Hash;
use pmfp_collections::{IRaz};

/// convenience trait for incremental test data
pub trait Adapt: 'static+Eq+Clone+Hash+Debug {}
impl<E> Adapt for E where E: 'static+Eq+Clone+Hash+Debug {}

/// common operations on sequences
pub trait IntrfSeq<T> {
	fn push(self, val:T) -> Self;
	fn pop(self) -> (Option<T>, Self);
}

impl<T:Adapt> IntrfSeq<T> for IRaz<T> {
	fn push(mut self, val:T) -> Self {
		self.push_left(val);
		self
	}
	fn pop(mut self) -> (Option<T>, Self) {
		let val = self.pop_left();
		(val, self)
	}
}

impl<T> IntrfSeq<T> for Vec<T> {
	fn push(mut self, val:T) -> Self {
		Vec::push(&mut self,val);
		self
	}
	fn pop(mut self) -> (Option<T>, Self) {
		let val = Vec::pop(&mut self);
		(val,self)
	}
}

