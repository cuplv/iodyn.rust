use rand::StdRng;
use Params;
use eval::*;

/// Test harness for `Vec`
///
/// Coordinates elements and insertion position
pub struct EvalVec<E:Eval,G:Rng> {
	vec: Vec<E>,
	coord: G,
}
impl<E:Eval,G:Rng>
EvalVec<E,G> {
	fn new(coord:G) -> Self {
		EvalVec {
			vec: Vec::new(),
			coord: coord,
		}
	}
}

/// Creates a `Vec` by pushing individual elements into
/// an initially unallocated `Vec`.
impl<E:Eval,G:Rng+Clone>
InitSeq<G>
for EvalVec<E,G> {
	fn init(p: &Params, coord: &G, mut _rng: &mut StdRng) -> (Duration,Self) {
		let mut eval = EvalVec::new((*coord).clone());
		let data_iter = eval.coord.gen_iter::<E>().take(p.start).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for dat in data_iter {
				eval.vec.push(dat)
			}
		});
		(time,eval)
	}
}

/// Appends to a `Vec` "batch-at-once" by `Vec::append`
impl<E:Eval,G:Rng>
EditExtend for EvalVec<E,G> {
	fn extend(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let mut data_vec = self.coord.gen_iter::<E>().take(batch_size).collect();
		let time = Duration::span(||{
			self.vec.append(&mut data_vec);
		});
		(time,self)
	}
}

/// Appends to a `Vec` one item at a time
impl<E:Eval,G:Rng>
EditAppend for EvalVec<E,G> {
	fn append(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let data_vec = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for val in data_vec {
				self.vec.push(val);
			}
		});
		(time,self)
	}
}

impl<E:Eval,G:Rng>
EditInsert for EvalVec<E,G> {
	fn insert(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let data_vec = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let loc = self.coord.gen::<usize>() % self.vec.len();
		let time = Duration::span(||{
			for val in data_vec {
				self.vec.insert(loc,val);
			}
		});
		(time,self)
	}
}

impl<E:Eval+Ord,G:Rng>
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