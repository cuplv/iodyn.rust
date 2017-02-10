use std::cmp::{min,max};
use std::rc::Rc;
use rand::StdRng;
use adapton::engine::*;
use pmfp_collections::{IRaz, IRazTree};
use pmfp_collections::inc_tree_cursor::gen_level;
use Params;
use eval::*;

pub struct EvalIRaz<E:Eval,G:ItemGen<E>> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<IRazTree<E>>,
	names: usize,
	data: G,
	counter: usize, // for name/levels during edit
	glob: Params
}

impl<E: Eval,G:ItemGen<E>> EvalIRaz<E,G> {
	pub fn new(p: &Params, data:G) -> Self {
		EvalIRaz {
			raztree: None,
			names: 0,
			data: data,
			counter: 0,
			glob: (*p).clone(),
		}
	}
	pub fn next_name(&mut self) -> Name {
		let n = self.names;
		self.names += 1;
		name_of_usize(n)
	}
}

/// Creates a `IRazTree` buy inserting elements, levels, and names (pregenerated)
/// into an initially unallocated `IRaz`, and then unfocusing
// uses Params::{start,namesize,unitsize}
impl<E:Eval,G:ItemGen<E>>
InitSeq<G>
for EvalIRaz<E,G> {
	type Item = E;
	fn init(p: &Params, data: &G, mut rng: &mut StdRng) -> (Duration,Self)
	{
		let mut eval = EvalIRaz::new(p,data.clone());
		let mut raz = IRaz::new();
		// measure stuff
		let names = p.start/(p.namesize*p.unitsize); // integer division
		let levels = p.start / p.unitsize; 
		let nonames = p.start - (names*p.namesize*p.unitsize);
		let units = nonames / p.unitsize; // integer division
		let nounits = nonames - (units*p.unitsize);
		// pregenerate data
		let mut lev_vec = Vec::with_capacity(levels);
		for _ in 0..levels {
			lev_vec.push(gen_level(rng))
		}
		let mut name_vec = Vec::with_capacity(names*p.namesize);
		for _ in 0..names {
			for _ in 0..(p.namesize-1){
				// no name with these levels
				name_vec.push(None)
			}
			name_vec.push(Some(eval.next_name()));
		}
		let mut data_iter = eval.data.gen_count(p.start,p).into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut level_iter = lev_vec.into_iter();
		// time the creation (insert and unfocus)
		let time = Duration::span(||{
			for _ in 0..names {
				for _ in 0..p.namesize {
					for _ in 0..p.unitsize {
						raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
					}
					raz.archive_left(level_iter.next().unwrap_or_else(||panic!("init")), name_iter.next().unwrap_or_else(||panic!("init")));
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..p.unitsize {
					raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
				}
				raz.archive_left(level_iter.next().unwrap_or_else(||panic!("init")), None);
			}
			for _ in 0..nounits {
				raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
			}
			eval.raztree = Some(raz.unfocus());
		});
		(time,eval)
	}
}

// TODO: this may generate an unused name/level. It uses the +1 only in unchecked side cases
impl<E:Eval,G:ItemGen<E>>
EditInsert for EvalIRaz<E,G> {
	fn insert(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap_or_else(||panic!("raz empty"));
		let loc = rng.gen::<usize>() % tree.len();
		let mut raz = tree.focus(loc).unwrap_or_else(||panic!("bad edit location"));
		// pregenerate data
		let new_levels = batch_size / self.glob.unitsize;
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels + 1) {
			lev_vec.push(gen_level(rng))
		}
		let new_names = batch_size / (self.glob.namesize*self.glob.unitsize);
		let mut name_vec = Vec::with_capacity(new_names);
		for _ in 0..(new_names + 1) {
			name_vec.push(Some(self.next_name()));
		}
		let data_iter = self.data.gen_count(batch_size,&self.glob).into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut lev_iter = lev_vec.into_iter();
		// time insertions
		let time = Duration::span(||{
			for data in data_iter {
				raz.push_left(data);
				self.counter += 1;
				if self.counter % (self.glob.namesize*self.glob.unitsize) == 0 {
					raz.archive_left(lev_iter.next().expect("lev_name"),name_iter.next().expect("name"));
				} else if self.counter % self.glob.unitsize == 0 {
					raz.archive_left(lev_iter.next().expect("lev"),None);
				}
			}
			self.raztree = Some(raz.unfocus());
		});
		(time,self)		
	}
}

// TODO: this may generate an unused name/level. It uses the +1 only in unchecked side cases
impl<E:Eval,G:ItemGen<E>>
EditAppend for EvalIRaz<E,G> {
	fn append(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap_or_else(||panic!("raz empty"));
		let len = tree.len();
		let mut raz = tree.focus(len).unwrap_or_else(||panic!("bad length"));
		// pregenerate data
		let new_levels = batch_size / self.glob.unitsize;
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels + 1) {
			lev_vec.push(gen_level(rng))
		}
		let new_names = batch_size / (self.glob.namesize*self.glob.unitsize);
		let mut name_vec = Vec::with_capacity(new_names);
		for _ in 0..(new_names + 1) {
			name_vec.push(Some(self.next_name()));
		}
		let data_iter = self.data.gen_count(batch_size,&self.glob).into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut lev_iter = lev_vec.into_iter();
		// time insertions
		let time = Duration::span(||{
			for data in data_iter {
				raz.push_left(data);
				self.counter += 1;
				if self.counter % (self.glob.namesize*self.glob.unitsize) == 0 {
					raz.archive_left(lev_iter.next().expect("lev_name"),name_iter.next().expect("name"));
				} else if self.counter % self.glob.unitsize == 0 {
					raz.archive_left(lev_iter.next().expect("lev"),None);
				}
			}
			self.raztree = Some(raz.unfocus());
		});
		(time,self)		
	}
}

/// Appends to a `RazTree` by focusing to the end, pushing
/// data, levels, and names, then unfocusing
// uses (saved) Params::{namesize,unitsize}
// TODO: Buggy
impl<E:Eval,G:ItemGen<E>>
EditExtend for EvalIRaz<E,G> {
	fn extend(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap();
		let namesize = self.glob.namesize;
		let unitsize = self.glob.unitsize;

		// measure stuff
		let len = tree.len();
		let mut newelems = batch_size;
		// fill in the level
		let levelless = len % unitsize;
		let pre_elems = min(unitsize - levelless, newelems);
		let madelevel = if levelless + pre_elems == unitsize {1} else {0}; 
		newelems -= pre_elems;
		// fill in the name
		let nameless = len % (namesize*unitsize);
		let pre_levels = min(
			(namesize - nameless - pre_elems) / unitsize,
			newelems / unitsize
		);
		let madename = if
			nameless / unitsize + madelevel + pre_levels
			== namesize
			{1} else {0}
		;
		newelems -= pre_levels * unitsize;
		// add more names etc. like above
		let names = newelems /(namesize*unitsize);
		let new_levels = madelevel + pre_levels + (newelems / unitsize);
		let nonames = newelems - names;
		let units = nonames / unitsize;
		let nounits = nonames - units;

		// pregenerate data
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels) {
			lev_vec.push(gen_level(rng))
		}
		let mut name_vec = Vec::with_capacity(madename + names*namesize);
		if madename == 1 {name_vec.push(Some(self.next_name()))}
		for _ in 0..names {
			for _ in 0..(namesize-1){
				// no name with these levels
				name_vec.push(None)
			}
			name_vec.push(Some(self.next_name()));
		}
		let mut data_iter = self.data.gen_count(batch_size,&self.glob).into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut level_iter = lev_vec.into_iter();

		// time the append
		let time = Duration::span(||{
			// finish the last level, name
			let mut raz = tree.focus(len).expect("02");
			for _ in 0..pre_elems {
				raz.push_left(data_iter.next().expect("03"));
			}
			for _ in 0..pre_levels {
				raz.archive_left(level_iter.next().expect("04"), None);
				for _ in 0..unitsize {
					raz.push_left(data_iter.next().expect("05"));
				}
			}
			if madename == 1 {
				raz.archive_left(level_iter.next().expect("06"), name_iter.next().expect("07"))
			} else if madelevel == 1 {
				raz.archive_left(level_iter.next().expect("08"), None)
			}
			// add new elms, levels, names, like above
			for _ in 0..names {
				for _ in 0..namesize {
					for _ in 0..unitsize {
						raz.push_left(data_iter.next().expect("09"));
					}
					raz.archive_left(level_iter.next().expect("10"), name_iter.next().expect("11"));
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..unitsize {
					raz.push_left(data_iter.next().expect("12"));
				}
				raz.archive_left(level_iter.next().expect("13"), None);
			}
			for _ in 0..nounits {
				raz.push_left(data_iter.next().expect("14"));
			}
			self.raztree = Some(raz.unfocus());
		});
		(time,self)
	}
}

impl<E:Eval+Ord,G:ItemGen<E>>
CompMax for EvalIRaz<E,G> {
	type Target = Option<E>;
	fn seq_max(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut max_val = None;
		let time = Duration::span(||{
	    max_val = Some(clone.fold_up(Rc::new(|e:&E|e.clone()),Rc::new(|e1:E,e2:E|max(e1,e2))))
		});
		(time,max_val.unwrap())
	}
}

