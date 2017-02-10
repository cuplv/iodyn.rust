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
	start: usize,
	unitsize: usize,
	namesize: usize,
	edits: usize,
	batches: usize,
	changes: usize,
	trials: usize,
}

pub struct EditParams {
	loc: usize,
	batch_size: usize,
}

pub trait Eval: 'static+Eq+Clone+Hash+Debug {}
impl<E> Eval for E where E: 'static+Eq+Clone+Hash+Debug {}

pub trait ItemGen<E:Eval>: Clone {
	fn gen_item(&mut self, p: &Params) -> E;
	fn gen_count(&mut self, count: usize, p:&Params) -> Vec<E> {
		let mut data_vec = Vec::with_capacity(count);
		for _ in 0..p.start {
			data_vec.push(self.gen_item(p));
		}
		data_vec
	}
}

// primitive traits applied to evaluatable data
trait InitSeq<G:ItemGen<Self::Item>> {
	type Item: Eval;
	/// generate an initial sequence, based on Params
	fn init(p: &Params, item_gen: &G, rng: &mut StdRng) -> (Duration,Self);
}
trait EditAppend {
	fn append(self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self);
}
trait EditInsert {
	fn insert(self, loc: usize, batch_size: usize, rng: &mut StdRng) -> (Duration,Self);
}
trait CompMax {
	type Target;
	fn seq_max(&self, rng: &mut StdRng) -> (Duration,Self::Target);
}

// General traits for types that perform some of the primitive actions above
trait Creator<R,D> {
	fn create(&mut self, rnd: &mut StdRng) -> (R,D);
}
trait Editor<R,D> {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (R,D);
}
trait Computor<R,D> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> R;
}
// combines everything
trait Testor<R> {
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
