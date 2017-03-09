use std::fmt::{self,Debug};
use std::rc::Rc;
use rand::{StdRng,Rng,Rand};
use time::Duration;
use primitives::*;

/// Test harness for `Vec`
///
/// Coordinates elements and insertion position
#[derive(Clone)]
pub struct EvalVec<E,G:Rng> {
	vec: Vec<E>,
	coord: G,
}
impl<E:Debug,G:Rng> Debug for EvalVec<E,G> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{:?}",self.vec)
	}
}
impl<E,G:Rng>
EvalVec<E,G> {
	fn new(coord:G) -> Self {
		EvalVec {
			vec: Vec::new(),
			coord: coord,
		}
	}
}

impl<E,G:Rng+Clone>
CreateEmpty<G> for EvalVec<E,G>{
	fn inc_empty(_unitgauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalVec::new((*coord).clone()));
		});
		(time,eval.unwrap())
	}
}
/// Creates a `Vec` by pushing individual elements into
/// an initially unallocated `Vec`. Ignores the incremental vars.
impl<E:Rand,G:Rng+Clone>
CreateInc<G>
for EvalVec<E,G> {
	fn inc_init(size: usize, _unigauge: usize, _namegauge: usize, coord: &G, mut _rng: &mut StdRng) -> (Duration,Self) {
		let mut eval = EvalVec::new((*coord).clone());
		let data_iter = eval.coord.gen_iter::<E>().take(size).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for dat in data_iter {
				eval.vec.push(dat)
			}
		});
		(time,eval)
	}
}

/// Appends to a `Vec` "batch-at-once" by `Vec::append`
impl<E:Rand,G:Rng>
EditExtend for EvalVec<E,G> {
	fn extend(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let mut data_vec = self.coord.gen_iter::<E>().take(batch_size).collect();
		let time = Duration::span(||{
			self.vec.append(&mut data_vec);
		});
		(time,self)
	}
}

/// Appends to a `Vec` one item at a time
impl<E:Rand,G:Rng>
EditAppend for EvalVec<E,G> {
	fn append(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let data_vec = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for val in data_vec {
				self.vec.push(val);
			}
		});
		(time,self)
	}
}

impl<E:Rand,G:Rng>
EditInsert for EvalVec<E,G> {
	fn insert(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let data_vec = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let loc = self.coord.gen::<usize>() % self.vec.len();
		let time = Duration::span(||{
			for val in data_vec {
				self.vec.insert(loc,val);
			}
		});
		(time,self)
	}
}

impl<E:Clone+Ord,G:Rng>
CompMax for EvalVec<E,G> {
	type Target = Option<E>;
	fn comp_max(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut max = None;
		let time = Duration::span(||{
			max = Some(self.vec.iter().max());
		});
		(time, max.unwrap().map(|e|(*e).clone()))
	}
}

// TODO: implement tree fold as multiple pairwise passes?

impl<E,O,F,G:Rng>
CompMap<E,O,F> for EvalVec<E,G> where
	F:Fn(&E)->O
{
	type Target = Vec<O>;
	fn comp_map(&self, f:Rc<F>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut mapped = None;
		let time = Duration::span(||{
			mapped = Some(self.vec.iter().map(|e|f(e)).collect())
		});
		(time, mapped.unwrap())
	}
}

impl<E,O,F,G:Rng>
CompFold<E,O,F> for EvalVec<E,G> where
	F:Fn(O,&E)->O,
{
	type Target = O;
	fn comp_fold(&self, accum: O, f:Rc<F>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut res = None;
		let time = Duration::span(||{
			res = Some(self.vec.iter().fold(accum,|o,e|f(o,e)));
		});
		(time, res.unwrap())
	}
}

