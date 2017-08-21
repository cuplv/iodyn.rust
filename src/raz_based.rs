use std::hash::{Hash};
use std::fmt::{Debug};
use raz::{Raz};
use raz_meta::{Position,FirstLast};
use adapton::engine::{Name};

/// A queue based on the Raz that refocuses each time
/// the operation on it switches between input and output.
pub struct Queue<E:Debug+Clone+Eq+Hash+'static>{
	// internally, the data is left to right in the order
	// it will be read.
	repr: Raz<E,()>,
	ready_to_dequeue: bool,
}

impl<E:Debug+Clone+Eq+Hash+'static> Queue<E> {
	pub fn new() -> Self {
		Queue{repr:Raz::new(), ready_to_dequeue:true}
	}
	pub fn enqueue(self, val: E) -> Self {
		let Queue{repr,ready_to_dequeue} = self;
		let mut raz = if ready_to_dequeue { 
			repr.unfocus().focus(Position::last()).unwrap()
		} else { repr };
		raz.push_left(val);
		Queue{repr:raz, ready_to_dequeue:false}
	}
	pub fn dequeue(self) -> (Self, Option<E>) {
		let Queue{repr,ready_to_dequeue} = self;
		let mut raz = if !ready_to_dequeue {
			repr.unfocus().focus(Position::first()).unwrap()
		} else { repr };
		let val = raz.pop_right();
		(Queue{repr:raz, ready_to_dequeue:true},val)
	}
	pub fn archive(self, nm: Name) -> Self {
		let Queue{repr,ready_to_dequeue} = self;
		let mut raz = if ready_to_dequeue {
			repr.unfocus().focus(Position::last()).unwrap()
		} else { repr };
		raz.archive_left(::inc_level(), Some(nm));
		Queue{repr:raz, ready_to_dequeue:false}
	}
	pub fn dequeue_name(self) -> (Self, Option<(E,Option<Name>)>) {
		let Queue{repr,ready_to_dequeue} = self;
		let mut raz = if !ready_to_dequeue {
			repr.unfocus().focus(Position::first()).unwrap()
		} else { repr };
		let pop = raz.pop_right_level_name();
		let ret = pop.map(|(v,l)|{(v, match l {
			None => None,
			Some((_,None)) => None,
			Some((_,n)) => n,
		})});
		(Queue{repr:raz, ready_to_dequeue:true},ret)
	}
}

/// A queue based on the Raz that uses the raz cursor,
/// refocusing only for the first enqueued item
/// since the last refocus.
pub struct ZipQueue<E:Debug+Clone+Eq+Hash+'static>{
	// internally, the raz cursor marks the read head, and
	// moves left to right, pushing behind and poping ahead
	repr: Raz<E,()>,
}

impl<E:Debug+Clone+Eq+Hash+'static> ZipQueue<E> {
	pub fn new() -> Self {
		ZipQueue{repr:Raz::new()}
	}
	pub fn enqueue(self, val: E) -> Self {
		let ZipQueue{mut repr} = self;
		repr.push_left(val);
		ZipQueue{repr:repr}
	}
	pub fn dequeue(self) -> (Self, Option<E>) {
		let ZipQueue{repr} = self;
		let mut raz = repr;
		let mut val = raz.pop_right();
		if val.is_none() {
			raz = raz.unfocus().focus(Position::first()).unwrap();
			val = raz.pop_right();
		}
		(ZipQueue{repr:raz},val)
	}
	pub fn archive(self, nm: Name) -> Self {
		let ZipQueue{mut repr} = self;
		repr.archive_left(::inc_level(), Some(nm));
		ZipQueue{repr:repr}
	}
	pub fn dequeue_name(self) -> (Self, Option<(E,Option<Name>)>) {
		let ZipQueue{repr} = self;
		let mut raz = repr;
		let mut pop = raz.pop_right_level_name();
		if pop.is_none() {
			raz = raz.unfocus().focus(Position::first()).unwrap();
			pop = raz.pop_right_level_name();
		}
		let ret = pop.map(|(v,l)|{(v, match l {
			None => None,
			Some((_,None)) => None,
			Some((_,n)) => n,
		})});
		(ZipQueue{repr:raz},ret)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_queue() {
		let q =
			Queue::new()
			.enqueue(1)
			.enqueue(2)
			.enqueue(3)
		;
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(1));
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(2));
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(3));
		let (q,val) = q.dequeue();
		assert_eq!(val, None);

		let q = q.enqueue(4);
		let q = q.enqueue(5);
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(4));
		let q = q.enqueue(6);
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(5));
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(6));
		let (q,val) = q.dequeue();
		assert_eq!(val, None);
	}

	#[test]
	fn test_zipqueue() {
		let q =
			ZipQueue::new()
			.enqueue(1)
			.enqueue(2)
			.enqueue(3)
		;
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(1));
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(2));
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(3));
		let (q,val) = q.dequeue();
		assert_eq!(val, None);

		let q = q.enqueue(4);
		let q = q.enqueue(5);
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(4));
		let q = q.enqueue(6);
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(5));
		let (q,val) = q.dequeue();
		assert_eq!(val, Some(6));
		let (q,val) = q.dequeue();
		assert_eq!(val, None);
	}
}