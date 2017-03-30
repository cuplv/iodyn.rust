use interface::*;
use std::rc::Rc;
use adapton::engine::*;

#[derive(Debug,PartialEq,Eq,Hash,Clone)]
pub enum List<X> {
	Nil,
	Cons(X, Box<List<X>>),
	Name(Name, Box<List<X>>),
	Art(Art<List<X>>),
}

impl<T:Adapt> IFaceSeq<T> for List<T> {
	fn seq_push(self, val:T) -> Self {
		List::Cons(val,Box::new(self))
	}
	fn seq_pop(self) -> (Option<T>, Self) {
		match self {
			List::Nil => (None, List::Nil),
			List::Cons(v,bl) => (Some(v),*bl),
			List::Name(_,bl) => bl.seq_pop(),
			List::Art(al) => force(&al).seq_pop(),
		} 
	}
}

impl<T:Adapt> IFaceArchive<(u32,Option<Name>)> for List<T> {
	fn archive(self, (_l,n):(u32,Option<Name>)) -> Self {
		match n {
			None => self,
			Some(n) => List::Name(n.clone(),Box::new(List::Art(cell(n,self))))
		}
	}
}

impl<T:Adapt> IFaceNew for List<T> {
	fn new() -> Self {
		List::Nil
	}
}

#[derive(Debug,PartialEq,Eq,Hash,Clone)]
pub enum RcList<X:Adapt> {
	Nil,
	Cons(X, Rc<RcList<X>>),
	Name(Name, Rc<RcList<X>>),
	Art(Art<Rc<RcList<X>>>),
}

impl<T:Adapt> IFaceSeq<T> for RcList<T> {
	fn seq_push(self, val:T) -> Self {
		RcList::Cons(val,Rc::new(self))
	}
	fn seq_pop(self) -> (Option<T>, Self) {
		match self {
			RcList::Nil => (None, self.clone()),
			RcList::Cons(v,bl) => (Some(v),(*bl).clone()),
			RcList::Name(_,bl) => (*bl).clone().seq_pop(),
			RcList::Art(al) => (*force(&al)).clone().seq_pop(),
		} 
	}
}

impl<T:Adapt> IFaceArchive<(u32,Option<Name>)> for RcList<T> {
	fn archive(self, (_l,n):(u32,Option<Name>)) -> Self {
		match n {
			None => self,
			Some(n) => RcList::Name(n.clone(),Rc::new(RcList::Art(cell(n,Rc::new(self)))))
		}
	}
}

impl<T:Adapt> IFaceNew for RcList<T> {
	fn new() -> Self {
		RcList::Nil
	}
}

#[derive(Debug,PartialEq,Eq,Hash,Clone)]
pub enum VecList<X> {
	Nil,
	Cons(Vec<X>, Box<VecList<X>>),
	Name(Name, Box<VecList<X>>),
	Art(Art<VecList<X>>),
}

impl<T:Adapt> IFaceSeq<T> for VecList<T> {
	fn seq_push(self, val:T) -> Self {
		match self {
			VecList::Cons(mut v,bl) => {
				v.push(val);
				VecList::Cons(v,bl)
			},
			_ => VecList::Cons(vec![val],Box::new(self))
		}
	}
	fn seq_pop(self) -> (Option<T>, Self) {
		match self {
			VecList::Nil => (None, VecList::Nil),
			VecList::Cons(mut vec,bl) => {
				let v = vec.pop().unwrap();
				if vec.is_empty() {
					(Some(v),*bl)
				} else {
					(Some(v),VecList::Cons(vec,bl))
				} 
			},
			VecList::Name(_,bl) => bl.seq_pop(),
			VecList::Art(al) => force(&al).seq_pop(),
		} 
	}
}

impl<T:Adapt> IFaceArchive<(u32,Option<Name>)> for VecList<T> {
	fn archive(self, (_l,n):(u32,Option<Name>)) -> Self {
		match n {
			None => self,
			Some(n) => VecList::Name(n.clone(),Box::new(VecList::Art(cell(n,self))))
		}
	}
}

impl<T:Adapt> IFaceNew for VecList<T> {
	fn new() -> Self {
		VecList::Nil
	}
}

#[derive(Debug,PartialEq,Eq,Hash,Clone)]
pub enum RcVecList<X:Adapt> {
	Nil,
	Cons(Vec<X>, Rc<RcVecList<X>>),
	Name(Name, Rc<RcVecList<X>>),
	Art(Art<Rc<RcVecList<X>>>),
}

impl<T:Adapt> IFaceSeq<T> for RcVecList<T> {
	fn seq_push(self, val:T) -> Self {
		match self {
			RcVecList::Cons(mut v,bl) =>{
				v.push(val);
				RcVecList::Cons(v,bl.clone())
			},
			_ => RcVecList::Cons(vec![val],Rc::new(self))
		}
	}
	fn seq_pop(self) -> (Option<T>, Self) {
		match self {
			RcVecList::Nil => (None, self.clone()),
			RcVecList::Cons(mut vec,bl) => {
				let v = vec.pop().unwrap();
				if vec.is_empty() {
					(Some(v),(*bl).clone())
				} else {
					(Some(v),RcVecList::Cons(vec,bl))
				} 
			},
			RcVecList::Name(_,ref bl) => (**bl).clone().seq_pop(),
			RcVecList::Art(ref al) => (*force(al)).clone().seq_pop(),
		} 
	}
}

impl<T:Adapt> IFaceArchive<(u32,Option<Name>)> for RcVecList<T> {
	fn archive(self, (_l,n):(u32,Option<Name>)) -> Self {
		match n {
			None => self,
			Some(n) => RcVecList::Name(n.clone(),Rc::new(RcVecList::Art(cell(n,Rc::new(self)))))
		}
	}
}

impl<T:Adapt> IFaceNew for RcVecList<T> {
	fn new() -> Self {
		RcVecList::Nil
	}
}

