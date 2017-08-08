use std::hash::{Hash};
use std::fmt::{Debug};
use raz::{Raz};
use raz_meta::{Position,FirstLast};
use adapton::engine::{Name};

pub struct Queue<E:Debug+Clone+Eq+Hash+'static>{
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
	pub fn dequeue_name(self) -> (Self, Option<(E,Option<Option<Name>>)>) {
		let Queue{repr,ready_to_dequeue} = self;
		let mut raz = if !ready_to_dequeue {
			repr.unfocus().focus(Position::first()).unwrap()
		} else { repr };
		let pop = raz.pop_right_level_name();
		let ret = pop.map(|(v,l)|{(v,l.map(|(_,n)|{n}))});
		(Queue{repr:raz, ready_to_dequeue:true},ret)
	}
}