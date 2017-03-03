use std::rc::Rc;
use rand::{StdRng,Rng};
use time::Duration;
use primitives::*;

/// Test harness for `Vec`
///
/// Coordinates elements and insertion position
#[derive(Clone)]
pub struct EvalVec<E:Adapt,G:Rng> {
	vec: Vec<E>,
	coord: G,
}
impl<E:Adapt,G:Rng>
EvalVec<E,G> {
	fn new(coord:G) -> Self {
		EvalVec {
			vec: Vec::new(),
			coord: coord,
		}
	}
}

impl<E:Adapt,G:Rng+Clone>
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
impl<E:Eval,G:Rng+Clone>
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
impl<E:Eval,G:Rng>
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
impl<E:Eval,G:Rng>
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

impl<E:Eval,G:Rng>
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

impl<E:Adapt,G:Rng>
EditSeq<E> for EvalVec<E,G> {
	fn push(mut self, val:E, _rng: &mut StdRng) -> (Duration, Self) {
		let time = Duration::span(||{
			self.vec.push(val);
		});
		(time,self)
	}
	fn pop(mut self, _rng: &mut StdRng) -> (Duration, Option<E>, Self) {
		let mut result = None;
		let time = Duration::span(||{
			result = self.vec.pop();
		});
		(time,result,self)		
	}
}

impl<E:Eval+Ord,G:Rng>
CompMax for EvalVec<E,G> {
	type Target = Option<E>;
	fn comp_max(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut max = None;
		let time = Duration::span(||{
			max = Some(self.vec.iter().max());
		});
		(time, max.unwrap().map(|ref e|(*e).clone()))
	}
}

// TODO: implement tree fold as multiple pairwise passes?

impl<E:Eval,O,F,G:Rng>
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

impl<E: Eval,O,F,G:Rng>
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

