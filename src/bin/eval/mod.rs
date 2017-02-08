pub mod eval_iraz;
pub mod eval_vec;
pub mod seq_test;

use std::fmt::Debug;
use std::hash::Hash;
use rand::{Rand, Rng, StdRng};
use time::Duration;

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

trait DataInit<'a,G:ItemGen<Self::Item>> {
	type Item: Eval;
	/// generate an initial sequence, based on Params
	fn init<'b>(p: &'a Params, data: G, rng: &'b mut StdRng) -> (Duration,Self);
}
trait EditAppend {
	fn edit(self, p: &EditParams, rng: &mut StdRng) -> (Duration,Self);
}
trait EditInsert {
	fn edit(self, p: &EditParams, rng: &mut StdRng) -> (Duration,Self);
}
trait CompMax {
	type Target;
	fn compute(&self, rng: &mut StdRng) -> (Duration,Self::Target);
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
