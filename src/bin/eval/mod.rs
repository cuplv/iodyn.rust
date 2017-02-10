pub mod actions;
pub mod eval_iraz;
pub mod eval_vec;
pub mod seq_test;

use std::fmt::Debug;
use std::hash::Hash;
use rand::{Rand, Rng, StdRng};
use time::Duration;

#[derive(Clone)]
pub struct Params {
	pub start: usize,
	pub unitsize: usize,
	pub namesize: usize,
	pub edits: usize,
	pub changes: usize,
	pub trials: usize,
}

pub trait Eval: 'static+Eq+Clone+Hash+Debug {}
impl<E> Eval for E where E: 'static+Eq+Clone+Hash+Debug {}

pub trait ItemGen<E:Eval>: Clone {
	fn gen_item(&mut self, p: &Params) -> E;
	fn gen_count(&mut self, count: usize, p:&Params) -> Vec<E> {
		let mut data_vec = Vec::with_capacity(count);
		for _ in 0..count {
			data_vec.push(self.gen_item(p));
		}
		data_vec
	}
}

// primitive traits applied to evaluatable data
/// for building an initial collection
pub trait InitSeq<G:ItemGen<Self::Item>> {
	type Item: Eval;
	fn init(p: &Params, item_gen: &G, rng: &mut StdRng) -> (Duration,Self);
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
	fn seq_max(&self, rng: &mut StdRng) -> (Duration,Self::Target);
}

// General traits for types that perform some of the primitive actions above
pub trait Creator<R,D> {
	fn create(&mut self, rnd: &mut StdRng) -> (R,D);
}
pub trait Editor<R,D> {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (R,D);
}
pub trait Computor<R,D> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> R;
}

// combines everything
pub trait Testor<R> {
	fn test(&mut self, rng: &mut StdRng) -> R;
}

/////////////////////
// Some Blanket Impls
/////////////////////

impl<E:Eval+Rand>
ItemGen<E> for StdRng {
	fn gen_item(&mut self, _p: &Params) -> E {
		Rng::gen::<E>(self)
	}
}
