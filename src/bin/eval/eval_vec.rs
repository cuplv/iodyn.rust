use rand::StdRng;
use Params;
use eval::*;

pub struct EvalVec<E:Eval,G:ItemGen<E>> {
	vec: Vec<E>,
	data: G,
	glob: Params,
}
impl<E:Eval,G:ItemGen<E>>
EvalVec<E,G> {
	fn new(p: &Params, data:G) -> Self {
		EvalVec {
			vec: Vec::new(),
			data: data,
			glob: (*p).clone(),
		}
	}
}

/// Creates a `Vec` by pushing individual elements into
/// an initially unallocated `Vec`.
impl<E:Eval,G:ItemGen<E>>
InitSeq<G>
for EvalVec<E,G> {
	type Item = E;
	fn init(p: &Params, data: &G, mut _rng: &mut StdRng) -> (Duration,Self) {
		let mut eval = EvalVec::new(p,data.clone());
		let data_iter = eval.data.gen_count(p.start,p).into_iter();
		let time = Duration::span(||{
			for dat in data_iter {
				eval.vec.push(dat)
			}
		});
		(time,eval)
	}
}

/// Appends to a `Vec` "batch-at-once" by `Vec::append`
impl<E:Eval,G:ItemGen<E>>
EditExtend for EvalVec<E,G> {
	fn extend(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let mut data_vec = self.data.gen_count(batch_size,&self.glob);
		let time = Duration::span(||{
			self.vec.append(&mut data_vec);
		});
		(time,self)
	}
}

impl<E:Eval,G:ItemGen<E>>
EditAppend for EvalVec<E,G> {
	fn append(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let data_vec = self.data.gen_count(batch_size,&self.glob);
		let time = Duration::span(||{
			for val in data_vec {
				self.vec.push(val);
			}
		});
		(time,self)
	}
}

impl<E:Eval,G:ItemGen<E>>
EditInsert for EvalVec<E,G> {
	fn insert(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let data_vec = self.data.gen_count(batch_size,&self.glob);
		let loc = rng.gen::<usize>() % self.vec.len();
		let time = Duration::span(||{
			for val in data_vec {
				self.vec.insert(loc,val);
			}
		});
		(time,self)
	}
}

impl<E:Eval+Ord,G:ItemGen<E>>
CompMax for EvalVec<E,G> {
	type Target = Option<E>;
	fn seq_max(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let mut max = None;
		let time = Duration::span(||{
	    max = Some(self.vec.iter().max());
	  });
	  (time, max.unwrap().map(|ref e|(*e).clone()))
	}
}
