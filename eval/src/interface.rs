use std::fmt::Debug;
use std::hash::Hash;
use pmfp_collections::{IRaz};
use adapton::engine::Name;

/// convenience trait for incremental test data
pub trait Adapt: 'static+Eq+Clone+Hash+Debug {}
impl<E> Adapt for E where E: 'static+Eq+Clone+Hash+Debug {}

/// common operations on sequences
pub trait IntrfSeq<T> {
	fn seq_push(self, val:T) -> Self;
	fn seq_pop(self) -> (Option<T>, Self);
}

/// archival structs take metadata
pub trait IntrfArchive<M> {
	fn archive(self, m:M) -> Self;
}

/// Polymorphic collection initializaton
pub trait IntrfNew {
	fn new() -> Self;
}

impl<T:Adapt> IntrfSeq<T> for IRaz<T> {
	fn seq_push(mut self, val:T) -> Self {
		self.push_left(val);
		self
	}
	fn seq_pop(mut self) -> (Option<T>, Self) {
		let val = self.pop_left();
		(val, self)
	}
}

impl<T:Adapt> IntrfArchive<(u32,Option<Name>)> for IRaz<T> {
	fn archive(mut self, (l,n):(u32,Option<Name>)) -> Self {
		self.archive_left(l,n);
		self
	}
}

impl<T:Adapt> IntrfNew for IRaz<T> {
	fn new() -> Self {
		IRaz::new()
	}
}

impl<T> IntrfSeq<T> for Vec<T> {
	fn seq_push(mut self, val:T) -> Self {
		self.push(val);
		self
	}
	fn seq_pop(mut self) -> (Option<T>, Self) {
		let val = self.pop();
		(val,self)
	}
}

impl<T> IntrfNew for Vec<T> {
	fn new() -> Self {
		Vec::new()
	}
}

