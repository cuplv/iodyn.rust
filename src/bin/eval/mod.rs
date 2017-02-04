pub mod eval_iraz;
pub mod eval_vec;
pub mod triple_test;

use std::fmt::Debug;
use std::hash::Hash;
use rand::{Rand, Rng, StdRng};
use time::Duration;
use Params;

pub struct EditParams {
	loc: usize,
	batch_size: usize,
}

pub trait Eval: 'static+Eq+Clone+Hash+Debug {}
impl<E> Eval for E where E: 'static+Eq+Clone+Hash+Debug {}

pub trait ElemGen<E:Eval> {
	fn gen(&mut self, p: &Params) -> E;
	fn gen_count(&mut self, count: usize, p:&Params) -> Vec<E> {
		let mut data_vec = Vec::with_capacity(count);
		for _ in 0..p.start {
			data_vec.push(self.gen(p));
		}
		data_vec
	}
}

trait DataInit<'a,'b,E:Eval,G:ElemGen<E>> {
	fn init(p: &'a Params, data: G, rng: &'b mut StdRng) -> (Duration,Self);
}
trait DataAppend {
	fn edit(self, p: &EditParams, rng: &mut StdRng) -> (Duration,Self);
}
trait DataInsert {
	fn edit(self, p: &EditParams, rng: &mut StdRng) -> (Duration,Self);
}
trait DataMax<E:Eval+Ord> {
	type Target;
	fn compute(&self, rng: &mut StdRng) -> (Duration,Self::Target);
}

trait Tester<'a,'b>: Sized {
	fn init(&mut self, &'a Params, &'b mut StdRng) -> Vec<Duration>;
	fn edit(&mut self, &EditParams, &mut StdRng) -> Vec<Duration>;
	fn run(&mut self, &mut StdRng) -> Vec<Duration>;
}

/////////////////////
// Some Blanket Impls
/////////////////////

impl<E:Eval+Rand>
ElemGen<E> for StdRng {
	fn gen(&mut self, _p: &Params) -> E {
		Rng::gen::<E>(self)
	}
}
