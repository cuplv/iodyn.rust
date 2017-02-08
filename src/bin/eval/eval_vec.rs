use rand::StdRng;
use Params;
use eval::*;

pub struct EvalVec<'a, E:Eval,G:ItemGen<E>> {
	vec: Vec<E>,
	data: G,
	glob: &'a Params,
}
impl<'a, E:Eval,G:ItemGen<E>>
EvalVec<'a, E,G> {
	fn new(p: &'a Params, data:G) -> Self {
		EvalVec {
			vec: Vec::new(),
			data: data,
			glob: p,
		}
	}
}

/// Creates a `Vec` by pushing individual elements into
/// an initially unallocated `Vec`.
// uses Params::{start} 
impl<'a, E:Eval,G:ItemGen<E>>
DataInit<'a,G>
for EvalVec<'a, E,G> {
	type Item = E;
	fn init<'b>(p: &'a Params, data: G, mut _rng: &'b mut StdRng) -> (Duration,Self) {
		let mut eval = EvalVec::new(p,data);
		let mut data_iter = eval.data.gen_count(p.start,p).into_iter();
		let time = Duration::span(||{
			for dat in data_iter {
				eval.vec.push(dat)
			}
		});
		(time,eval)
	}
}

/// Appends to a `Vec` "batch-at-once" by `Vec::append`
// uses EditPArams::{batch_size} 
impl<'a, E:Eval,G:ItemGen<E>>
EditAppend for EvalVec<'a, E,G> {
	fn edit(mut self, p: &EditParams, _rng: &mut StdRng) -> (Duration,Self) {
		let mut data_vec = self.data.gen_count(p.batch_size,self.glob);
		let time = Duration::span(||{
			self.vec.append(&mut data_vec);
		});
		(time,self)
	}
}

impl<'a, E:Eval+Ord,G:ItemGen<E>>
CompMax for EvalVec<'a, E,G> {
	type Item = E;
	type Target = Option<E>;
	fn compute(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut max = None;
		let time = Duration::span(||{
	    max = Some(self.vec.iter().max());
	  });
	  (time, max.unwrap().map(|ref e|(*e).clone()))
	}
}
