pub mod eval_iraz;
pub mod eval_vec;
pub mod triple_test;

use std::fmt::Debug;
use std::hash::Hash;
use rand::{Rand, Rng, StdRng};
use time::Duration;
use Params;

pub trait Eval: 'static+Eq+Clone+Hash+Debug {}
impl<E> Eval for E where E: 'static+Eq+Clone+Hash+Debug {}

pub trait ElemGen<E:Eval> {
	fn gen(&mut self, p: &Params) -> E;
}

trait DataInit<E:Eval,G:ElemGen<E>> { //: Sized {
	fn init(p: &Params, data: G, rng: &mut StdRng) -> Self;
}
trait DataAppend {
	fn append(self, p: &Params, rng: &mut StdRng) -> Self;
}
trait DataInsert {
	fn insert(self, pos: usize, p: &Params, rng: &mut StdRng) -> Self;
}
trait DataMax<E:Eval+Ord> {
	fn max(&self, rng: &mut StdRng) -> Option<E>;
}

trait Tester: Sized {
	fn init(&mut self, &Params, &mut StdRng) -> Vec<Duration>;
	fn edit(&mut self, &Params, &mut StdRng) -> Vec<Duration>;
	fn run(&mut self, &Params, &mut StdRng) -> Vec<Duration>;
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
