//use std::cmp::{min,max};
//use std::rc::Rc;
use rand::StdRng;
//use adapton::engine::*;
use pmfp_collections::{Raz, RazTree};
use pmfp_collections::trees::NegBin;
use eval::*;

/// Test harness for the incremental Raz
///
/// Coorinates elements and insertion location
#[allow(unused)]
pub struct EvalNRaz<E:Eval,G:Rng> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<RazTree<E>>,
	coord: G,
	//counter: usize, // for name/levels during edit
	unitsize: usize,
}

impl<E: Eval,G:Rng> EvalNRaz<E,G> {
	pub fn new(us: usize, coord:G) -> Self {
		EvalNRaz {
			raztree: None,
			coord: coord,
			//counter: 0,
			unitsize: us,
		}
	}
}

/// Creates a `RazTree` buy inserting elements, levels, and names (pregenerated)
/// into an initially unallocated `Raz`, and then unfocusing
impl<E:Eval,G:Rng+Clone>
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

