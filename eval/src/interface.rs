use std::fmt::Debug;
use std::hash::Hash;
use pmfp_collections::{IRaz};
use pmfp_collections::inc_archive_stack::AStack as IAStack;
use adapton::engine::Name;

/// convenience trait for incremental test data
pub trait Adapt: 'static+Eq+Clone+Hash+Debug {}
impl<E> Adapt for E where E: 'static+Eq+Clone+Hash+Debug {}

/// common operations on sequences
pub trait IFaceSeq<T> {
	fn seq_push(self, val:T) -> Self;
	fn seq_pop(self) -> (Option<T>, Self);
}

/// archival structs take metadata
pub trait IFaceArchive<M> {
	fn archive(self, m:M) -> Self;
}

/// Polymorphic collection initializaton
pub trait IFaceNew {
	fn new() -> Self;
}

//////////
// IRaz
/////////

impl<T:Adapt> IFaceSeq<T> for IRaz<T> {
	fn seq_push(mut self, val:T) -> Self {
		self.push_left(val);
		self
	}
	fn seq_pop(mut self) -> (Option<T>, Self) {
		let val = self.pop_left();
		(val, self)
	}
}

impl<T:Adapt> IFaceArchive<(u32,Option<Name>)> for IRaz<T> {
	fn archive(mut self, (l,n):(u32,Option<Name>)) -> Self {
		self.archive_left(l,n);
		self
	}
}

impl<T:Adapt> IFaceNew for IRaz<T> {
	fn new() -> Self {
		IRaz::new()
	}
}

//////////
// IAStack
/////////

impl<T:Adapt,M:Adapt> IFaceSeq<T> for IAStack<T,M> {
	fn seq_push(mut self, val:T) -> Self {
		self.push(val);
		self
	}
	fn seq_pop(mut self) -> (Option<T>, Self) {
		let val = self.pop();
		(val, self)
	}
}

impl<T:Adapt,M:Adapt> IFaceArchive<(M,Option<Name>)> for IAStack<T,M> {
	fn archive(mut self, (m,n):(M,Option<Name>)) -> Self {
		IAStack::archive(&mut self,n,m);
		self
	}
}

impl<T:Adapt,M:Adapt> IFaceNew for IAStack<T,M> {
	fn new() -> Self {
		IAStack::new()
	}
}

//////////
// Vec
/////////

impl<T> IFaceSeq<T> for Vec<T> {
	fn seq_push(mut self, val:T) -> Self {
		self.push(val);
		self
	}
	fn seq_pop(mut self) -> (Option<T>, Self) {
		let val = self.pop();
		(val,self)
	}
}

impl<T,M> IFaceArchive<M> for Vec<T> {
	/// Vec isn't an archival struct, so do nothing
	fn archive(self, _m:M) -> Self { self }
}

impl<T> IFaceNew for Vec<T> {
	fn new() -> Self {
		Vec::new()
	}
}

