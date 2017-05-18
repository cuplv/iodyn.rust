use std::rc::Rc;
use rand::{Rng,StdRng};
use time::Duration;
use adapton::engine::*;

/// empty initialization of an incremental collection test harness
pub trait CreateEmpty<G:Rng> {
	fn inc_empty(datagauge: usize, namegauge: usize, coord: &G, rng: &mut StdRng) -> (Duration, Self);
}
/// initialization of test harness from provided data
pub trait CreateFrom<T,G:Rng> {
	fn inc_from(data: T, datagauge: usize, namegauge: usize, coord: &G, rng: &mut StdRng) -> (Duration, Self);
}
/// for building a test harness from randomized data
pub trait CreateInc<G:Rng> {
	fn inc_init(size: usize, datagauge: usize, namegauge: usize, coord: &G, rng: &mut StdRng) -> (Duration,Self);
}
/// for adding elements as if initialization was longer
pub trait EditExtend {
	fn extend(self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self);
}
/// for adding elements as if the user is editing
pub trait EditAppend {
	fn append(self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self);
}
/// for inserting elements at random location
pub trait EditInsert {
	fn insert(self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self);
}
/// for computing the max of the collection
pub trait CompMax {
	type Target;
	fn comp_max(&self, rng: &mut StdRng) -> (Duration,Self::Target);
}

/// for reversing a sequence
pub trait CompRev {
	type Target;
	fn comp_rev(&self, rng: &mut StdRng) -> (Duration,Self::Target);
}

pub trait CompNative<O> {
	type Input;
	fn comp_native<F:Fn(&Self::Input)->O>(&self, f:Rc<F>, rng: &mut StdRng) -> (Duration,O);
}

pub trait CompTreeFold<R,O,I:Fn(&R)->O,B:Fn(O,O)->O> {
	type Target;
	fn comp_tfold(&self, init:Rc<I>, bin:Rc<B>, rng: &mut StdRng) -> (Duration,Self::Target);
}

pub trait CompTreeFoldNL<R,O,I:Fn(&R)->O,B:Fn(O,O)->O,M:Fn(O,u32,Option<Name>,O)->O> {
	type Target;
	fn comp_tfoldnl(&self, init:Rc<I>, bin:Rc<B>, binnl:Rc<M>, rng: &mut StdRng) -> (Duration,Self::Target);
}

pub trait CompTreeFoldG<R,O,I:Fn(&Vec<R>)->O,B:Fn(O,u32,Option<Name>,O)->O> {
	type Target;
	fn comp_tfoldg(&self, init:Rc<I>, bin:Rc<B>, rng: &mut StdRng) -> (Duration,Self::Target);
}

/// changes every value to another based on function
pub trait CompMap<I,O,F:Fn(&I)->O> {
	type Target;
	fn comp_map(&self, f:Rc<F>, rng: &mut StdRng) -> (Duration,Self::Target);
}

/// folds every element into the binary function, starting with the given one
pub trait CompFold<I,O,F:Fn(O,&I)->O> {
	type Target;
	fn comp_fold(&self, accum: O, f:Rc<F>, rng: &mut StdRng) -> (Duration,Self::Target);
}

/// folds every element into the binary function, or into meta data
pub trait CompFoldMeta<I,O,M,B:Fn(O,&I)->O,N:Fn(O,M)->O> {
	type Target;
	fn comp_fold_meta(&self, accum: O, b:Rc<B>, m:Rc<N>, rng: &mut StdRng) -> (Duration,Self::Target);
}

/// folds every element into the binary function, or into meta data
pub trait CompFoldArchive<I,O,M,B:Fn(O,&I)->O,FB:Fn(O,Option<Name>)->O,N:Fn(O,M)->O> {
	type Target;
	fn comp_fold_archive(&self, accum: O, b:Rc<B>, fb:Rc<FB>, m:Rc<N>, rng: &mut StdRng) -> (Duration,Self::Target);
}
