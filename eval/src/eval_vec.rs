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
	fn inc_empty(_datagauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalVec::new((*coord).clone()));
		});
		(time,eval.unwrap())
	}
}

impl<E,G:Rng+Clone>
CreateFrom<Vec<E>,G> for EvalVec<E,G>{
	fn inc_from(data: Vec<E>, _datagauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalVec{vec:data,coord:(*coord).clone()});
		});
		(time,eval.unwrap())
	}
}

/// Creates a `Vec` by pushing individual elements into
/// an initially unallocated `Vec`. Ignores the incremental vars.
impl<E:Rand,G:Rng+Clone>
CreateInc<G>
for EvalVec<E,G> {
	fn inc_init(size: usize, _datagauge: usize, _namegauge: usize, coord: &G, mut _rng: &mut StdRng) -> (Duration,Self) {
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

impl<E:Clone,G:Rng>
CompRev for EvalVec<E,G> {
	type Target = Vec<E>;
	fn comp_rev(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut rev = None;
		let time = Duration::span(||{
			let mut clone = self.vec.clone();
			clone.reverse();
			rev = Some(clone);
		});
		(time, rev.unwrap())
	}
}

impl<E:Clone,O,G:Rng>
CompNative<O> for EvalVec<E,G> {
	type Input = Vec<E>;
	fn comp_native<F>(&self, f:Rc<F>, _rng: &mut StdRng) -> (Duration,O) where
		F:Fn(&Self::Input)->O,
	{
		let mut nat = None;
		let time = Duration::span(||{
			nat = Some(f(&self.vec));
		});
		(time, nat.unwrap())
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

impl<E,O,M,F,N,G:Rng>
CompFoldMeta<E,O,M,F,N> for EvalVec<E,G> where
	F:Fn(O,&E)->O,
	N:Fn(O,M)->O,
{
	type Target = O;
	/// Vecs don't take or use metadata, so just fold
	fn comp_fold_meta(&self, accum: O, f:Rc<F>, _n:Rc<N>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut res = None;
		let time = Duration::span(||{
			res = Some(self.vec.iter().fold(accum,|o,e|f(o,e)));
		});
		(time, res.unwrap())
	}
}

