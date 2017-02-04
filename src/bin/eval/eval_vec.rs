use rand::StdRng;
use Params;
use eval::*;

pub struct EvalVec<E:Eval,G:ElemGen<E>> {
	vec: Vec<E>,
	data: G,
}
impl<E:Eval,G:ElemGen<E>> EvalVec<E,G> {
	fn new(data:G) -> Self {
		EvalVec {
			vec: Vec::new(),
			data: data,
		}
	}
}

impl<E:Eval,G:ElemGen<E>> DataInit<E,G> for EvalVec<E,G> {
	fn init(p: &Params, data: G, mut _rng: &mut StdRng) -> Self {
		let mut eval = EvalVec::new(data);
		for _ in 0..p.start {
			eval.vec.push(eval.data.gen(p))
		}
		eval
	}
}

impl<E:Eval,G:ElemGen<E>> DataAppend for EvalVec<E,G> {
	fn append(mut self, p: &Params, _rng: &mut StdRng) -> Self {
		self.vec.push(self.data.gen(p));
		self
	}
}

impl<E:Eval+Ord,G:ElemGen<E>> DataMax<E> for EvalVec<E,G> {
	fn max(&self, _rng: &mut StdRng) -> Option<E> {
    self.vec.iter().max().map(|ref e|(*e).clone())
	}
}
