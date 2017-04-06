//use std::cmp::{min,max};
//use std::rc::Rc;
use rand::{Rng,StdRng,Rand};
//use adapton::engine::*;
use time::Duration;
use pmfp_collections::{Raz, RazTree};
use pmfp_collections::trees::NegBin;
use primitives::*;
use interface::{Adapt};

/// Test harness for the incremental Raz
///
/// Coorinates elements and insertion location
#[allow(unused)]
pub struct EvalNRaz<E:Adapt,G:Rng> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<RazTree<E>>,
	coord: G,
	counter: usize, // for name/levels during edit
	unitsize: usize,
}

impl<E: Adapt,G:Rng> EvalNRaz<E,G> {
	pub fn new(us: usize, coord:G) -> Self {
		EvalNRaz {
			raztree: None,
			coord: coord,
			counter: 0,
			unitsize: us,
		}
	}
}

/// Creates a `RazTree` buy inserting elements, levels, and names (pregenerated)
/// into an initially unallocated `Raz`, and then unfocusing
impl<E:Adapt+Rand,G:Rng+Clone>
CreateInc<G>
for EvalNRaz<E,G> {
	fn inc_init(size: usize, unitgauge: usize, _namegauge: usize, coord: &G, mut rng: &mut StdRng) -> (Duration,Self)
	{
		let mut eval = EvalNRaz::new(unitgauge, (*coord).clone());
		let mut raz = Raz::new();
		// measure stuff
		let levels = size / eval.unitsize; 
		let nolevs = size - (levels*eval.unitsize);
		// pregenerate data
		let mut lev_vec = Vec::with_capacity(levels);
		for _ in 0..levels {
			lev_vec.push(rng.gen::<NegBin>())
		}
		let mut data_iter = eval.coord.gen_iter().take(size).collect::<Vec<_>>().into_iter();
		let mut level_iter = lev_vec.into_iter();
		// time the creation (insert and unfocus)
		let time = Duration::span(||{
			for _ in 0..levels {
				for _ in 0..eval.unitsize {
					raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
				}
				raz.archive_left(level_iter.next().unwrap_or_else(||panic!("init")));
			}
			for _ in 0..nolevs {
				raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
			}
			eval.raztree = Some(raz.unfocus());
		});
		(time,eval)
	}
}

// TODO: this may generate an unused name/level. It uses the +1 only in unchecked side cases
impl<E:Adapt+Rand,G:Rng>
EditInsert for EvalNRaz<E,G> {
	fn insert(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap_or_else(||panic!("raz uninitialized"));
		let loc = self.coord.gen::<usize>() % tree.len();
		let mut focus = None;
		let focus_time = Duration::span(||{
			focus = tree.focus(loc);
		});
		let mut raz = focus.unwrap_or_else(||panic!("bad edit location"));
		// pregenerate data
		let new_levels = batch_size / self.unitsize;
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels + 1) {
			lev_vec.push(rng.gen())
		}
		let data_iter = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let mut lev_iter = lev_vec.into_iter();
		// time insertions
		let insert_time = Duration::span(||{
			for data in data_iter {
				raz.push_left(data);
				self.counter += 1;
				if self.counter % self.unitsize == 0 {
					raz.archive_left(lev_iter.next().expect("lev"));
				}
			}
			self.raztree = Some(raz.unfocus());
		});
		(focus_time+insert_time,self)		
	}
}
