use std::rc::Rc;
use rand::{Rng,StdRng};
use time::Duration;

/// empty initialization of an incremental collection test harness
pub trait CreateEmpty<G:Rng> {
	fn inc_empty(unitgauge: usize, namegauge: usize, coord: &G, rng: &mut StdRng) -> (Duration, Self);
}
/// initialization of test harness from provided data
pub trait CreateFrom<T,G:Rng> {
	fn inc_from(data: T, unitgauge: usize, namegauge: usize, coord: &G, rng: &mut StdRng) -> (Duration, Self);
}
/// for building a test harness from randomized data
pub trait CreateInc<G:Rng> {
	fn inc_init(size: usize, unitgauge: usize, namegauge: usize, coord: &G, rng: &mut StdRng) -> (Duration,Self);
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

pub trait CompTreeFold<R,O,I:Fn(&R)->O,B:Fn(O,O)->O> {
	type Target;
	fn comp_tfold(&self, init:Rc<I>, bin:Rc<B>, rng: &mut StdRng) -> (Duration,Self::Target);
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
