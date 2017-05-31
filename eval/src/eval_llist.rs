use std::fmt::{self,Debug};
use std::rc::Rc;
use std::collections::LinkedList;
use rand::{StdRng,Rng,Rand};
use time::Duration;
use primitives::*;

/// Test harness for `Vec`
///
/// Coordinates elements and insertion position
#[derive(Clone)]
pub struct EvalLList<E,G:Rng> {
	list: LinkedList<E>,
	coord: G,
}
impl<E:Debug,G:Rng> Debug for EvalLList<E,G> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{:?}",self.list)
	}
}
impl<E,G:Rng>
EvalLList<E,G> {
	fn new(coord:G) -> Self {
		EvalLList {
			list: LinkedList::new(),
			coord: coord,
		}
	}
}


/// Creates a `LinkedList` by pushing individual elements into
/// an initially unallocated `LinkedList`. Ignores the incremental vars.
impl<E:Rand,G:Rng+Clone>
CreateInc<G>
for EvalLList<E,G> {
	fn inc_init(size: usize, _datagauge: usize, _namegauge: usize, coord: &G, mut _rng: &mut StdRng) -> (Duration,Self) {
		let mut eval = EvalLList::new((*coord).clone());
		let data_iter = eval.coord.gen_iter::<E>().take(size).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for dat in data_iter {
				eval.list.push_back(dat)
			}
		});
		(time,eval)
	}
}

impl<E:Rand,G:Rng>
EditInsert for EvalLList<E,G> {
	fn insert(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let loc = self.coord.gen::<usize>() % self.list.len();
		let data_vec = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for val in data_vec {
				let mut end = self.list.split_off(loc);
				self.list.push_back(val);
				self.list.append(&mut end);
			}
		});
		(time,self)
	}
}

impl<E,G:Rng,F:Fn(&mut G)->E>
EditInsertCustom<G,E,F> for EvalLList<E,G> {
	fn insert_custom(mut self, batch_size: usize, create_fn: &F, _rng: &mut StdRng) -> (Duration,Self) {
		let loc = self.coord.gen::<usize>() % self.list.len();
		let data_vec = (0..batch_size).map(|_|create_fn(&mut self.coord)).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for val in data_vec {
				let mut end = self.list.split_off(loc);
				self.list.push_back(val);
				self.list.append(&mut end);
			}
		});
		(time,self)
	}
}

impl<E:Clone,O,G:Rng>
CompNative<O> for EvalLList<E,G> {
	type Input = LinkedList<E>;
	fn comp_native<F>(&self, f:Rc<F>, _rng: &mut StdRng) -> (Duration,O) where
		F:Fn(&Self::Input)->O,
	{
		let mut nat = None;
		let time = Duration::span(||{
			nat = Some(f(&self.list));
		});
		(time, nat.unwrap())
	}
}
