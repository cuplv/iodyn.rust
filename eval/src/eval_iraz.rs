use std::cmp::{min,max};
use std::rc::Rc;
use rand::{Rng,StdRng,Rand};
use time::Duration;
use adapton::engine::*;
use pmfp_collections::{IRaz, IRazTree};
use pmfp_collections::inc_tree_cursor::gen_level;
use primitives::*;
use interface::{Adapt};

/// Test harness for the incremental Raz
///
/// Coorinates elements, insertion location, gen'ed levels
#[derive(Clone)]
pub struct EvalIRaz<E:Adapt,G:Rng> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<IRazTree<E>>,
	names: usize,
	coord: G,
	counter: usize, // for name/levels during edit
	unitsize: usize,
	namesize: usize,
}

impl<E:Adapt,G:Rng> EvalIRaz<E,G> {
	pub fn new(us: usize, ns: usize, coord:G) -> Self {
		EvalIRaz {
			raztree: None,
			names: 0,
			coord: coord,
			counter: 0,
			unitsize: us,
			namesize: ns,
		}
	}
	pub fn next_name(&mut self) -> Name {
		let n = self.names;
		self.names += 1;
		name_of_usize(n)
	}
}

impl<E:Adapt,G:Rng+Clone>
CreateEmpty<G> for EvalIRaz<E,G>{
	fn inc_empty(unitgauge: usize, namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = EvalIRaz::new(unitgauge, namegauge, (*coord).clone());
		let time = Duration::span(||{
			eval.raztree = Some(IRaz::new().unfocus());
		});
		(time,eval)
	}
}
/// Creates a `IRazTree` buy inserting elements, levels, and names (pregenerated)
/// into an initially unallocated `IRaz`, and then unfocusing
// uses Params::{start,namesize,unitsize}
impl<E:Adapt+Rand,G:Rng+Clone>
CreateInc<G> for EvalIRaz<E,G> {
	fn inc_init(size: usize, unitgauge: usize, namegauge: usize, coord: &G, mut rng: &mut StdRng) -> (Duration,Self)
	{
		let mut eval = EvalIRaz::new(unitgauge, namegauge, (*coord).clone());
		// measure stuff
		let names = size/(eval.namesize*eval.unitsize); // integer division
		let levels = size / eval.unitsize; 
		let nonames = size - (names*eval.namesize*eval.unitsize);
		let units = nonames / eval.unitsize; // integer division
		let nounits = nonames - (units*eval.unitsize);
		// pregenerate data
		let mut lev_vec = Vec::with_capacity(levels);
		for _ in 0..levels {
			lev_vec.push(gen_level(rng))
		}
		let mut name_vec = Vec::with_capacity(names*eval.namesize);
		for _ in 0..names {
			for _ in 0..(eval.namesize-1){
				// no name with these levels
				name_vec.push(None)
			}
			name_vec.push(Some(eval.next_name()));
		}
		let mut data_iter = eval.coord.gen_iter().take(size).collect::<Vec<_>>().into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut level_iter = lev_vec.into_iter();
		// time the creation (insert and unfocus)
		let time = Duration::span(||{
			let mut raz = IRaz::new();
			for _ in 0..names {
				for _ in 0..eval.namesize {
					for _ in 0..eval.unitsize {
						raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
					}
					raz.archive_left(level_iter.next().unwrap_or_else(||panic!("init")), name_iter.next().unwrap_or_else(||panic!("init")));
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..eval.unitsize {
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
impl<E:Adapt+Rand,G:Rng>
EditInsert for EvalIRaz<E,G> {
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
			lev_vec.push(gen_level(rng))
		}
		let new_names = batch_size / (self.namesize*self.unitsize);
		let mut name_vec = Vec::with_capacity(new_names);
		for _ in 0..(new_names + 1) {
			name_vec.push(Some(self.next_name()));
		}
		let data_iter = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut lev_iter = lev_vec.into_iter();
		// time insertions
		let insert_time = Duration::span(||{
			for data in data_iter {
				raz.push_left(data);
				self.counter += 1;
				if self.counter % (self.namesize*self.unitsize) == 0 {
					raz.archive_left(lev_iter.next().expect("lev_name"),name_iter.next().expect("name"));
				} else if self.counter % self.unitsize == 0 {
					raz.archive_left(lev_iter.next().expect("lev"),None);
				}
			}
			self.raztree = Some(raz.unfocus());
		});
		(focus_time+insert_time,self)		
	}
}

// TODO: this may generate an unused name/level. It uses the +1 only in unchecked side cases
impl<E:Adapt+Rand,G:Rng>
EditAppend for EvalIRaz<E,G> {
	fn append(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap_or_else(||panic!("raz uninitialized"));
		let len = tree.len();
		let mut focus = None;
		let focus_time = Duration::span(||{
			focus = tree.focus(len);
		});
		let mut raz = focus.unwrap_or_else(||panic!("bad edit location"));
		// pregenerate data
		let new_levels = batch_size / self.unitsize;
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels + 1) {
			lev_vec.push(gen_level(rng))
		}
		let new_names = batch_size / (self.namesize*self.unitsize);
		let mut name_vec = Vec::with_capacity(new_names);
		for _ in 0..(new_names + 1) {
			name_vec.push(Some(self.next_name()));
		}
		let data_iter = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut lev_iter = lev_vec.into_iter();
		// time insertions
		let time = Duration::span(||{
			for data in data_iter {
				raz.push_left(data);
				self.counter += 1;
				if self.counter % (self.namesize*self.unitsize) == 0 {
					raz.archive_left(lev_iter.next().expect("lev_name"),name_iter.next().expect("name"));
				} else if self.counter % self.unitsize == 0 {
					raz.archive_left(lev_iter.next().expect("lev"),None);
				}
			}
			self.raztree = Some(raz.unfocus());
		});
		(focus_time+time,self)		
	}
}

/// Appends to a `RazTree` by focusing to the end, pushing
/// data, levels, and names, then unfocusing
// uses (saved) Params::{namesize,unitsize}
// TODO: Buggy
impl<E:Adapt+Rand,G:Rng>
EditExtend for EvalIRaz<E,G> {
	fn extend(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap();

		// measure stuff
		let len = tree.len();
		let mut newelems = batch_size;
		// fill in the level
		let levelless = len % self.unitsize;
		let pre_elems = min(self.unitsize - levelless, newelems);
		let madelevel = if levelless + pre_elems == self.unitsize {1} else {0}; 
		newelems -= pre_elems;
		// fill in the name
		let nameless = len % (self.namesize*self.unitsize);
		let pre_levels = min(
			(self.namesize - nameless - pre_elems) / self.unitsize,
			newelems / self.unitsize
		);
		let madename = if
			nameless / self.unitsize + madelevel + pre_levels
			== self.namesize
			{1} else {0}
		;
		newelems -= pre_levels * self.unitsize;
		// add more names etc. like above
		let names = newelems /(self.namesize*self.unitsize);
		let new_levels = madelevel + pre_levels + (newelems / self.unitsize);
		let nonames = newelems - names;
		let units = nonames / self.unitsize;
		let nounits = nonames - units;

		// pregenerate data
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels) {
			lev_vec.push(gen_level(rng))
		}
		let mut name_vec = Vec::with_capacity(madename + names*self.namesize);
		if madename == 1 {name_vec.push(Some(self.next_name()))}
		for _ in 0..names {
			for _ in 0..(self.namesize-1){
				// no name with these levels
				name_vec.push(None)
			}
			name_vec.push(Some(self.next_name()));
		}
		let mut data_iter = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
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
				for _ in 0..self.unitsize {
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
				for _ in 0..self.namesize {
					for _ in 0..self.unitsize {
						raz.push_left(data_iter.next().expect("09"));
					}
					raz.archive_left(level_iter.next().expect("10"), name_iter.next().expect("11"));
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..self.unitsize {
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

// impl<E:Adapt,G:Rng>
// EditSeq<E> for EvalIRaz<E,G> {
// 	fn push(self, val:E) -> (Duration, Self) {
// 		// focus to end
// 		let tree = self.raztree.take().unwrap_or_else(||panic!("raz uninitialized"));
// 		let mut focus = None;
// 		let focus_time = Duration::span(||{
// 			focus = tree.focus(tree.len());
// 		});
// 		let mut raz = focus.unwrap();
// 		// determine if we need names/levels
// 		let lev = if self.counter % self.unitsize == 0 {
// 			Some(gen_level(self.coord))
// 		} else { None };
// 		let nm = if self.counter % (self.namesize*self.unitsize) == 0 {
// 			Some(self.next_name())
// 		} else { None };
// 		self.counter += 1;
// 		// do insertion/unfocus
// 		let insert_time = Duration::span(||{
// 			if let Some(lev) = lev {
// 				raz.archive_left(lev,nm);
// 			}
// 			raz.push_left(val);
// 			self.raztree = Some(raz.unfocus());
// 		});
// 		(focus_time+insert_time,self)		
// 	}
// 	fn pop(self) -> (Duration, Option<E>, Self) {
// 		// focus to end
// 		let tree = self.raztree.take().unwrap_or_else(||panic!("raz uninitialized"));
// 		let mut focus = None;
// 		let focus_time = Duration::span(||{
// 			focus = tree.focus(tree.len());
// 		});
// 		let mut raz = focus.unwrap();
// 		// do removal/unfocus
// 		let result = None;
// 		let remove_time = Duration::span(||{
// 			result = raz.pop_left();
// 			self.raztree = Some(raz.unfocus());
// 		});
// 		(focus_time+remove_time,result,self)		
// 	}
// }

impl<E:Adapt+Ord,G:Rng>
CompMax for EvalIRaz<E,G> {
	type Target = Option<E>;
	fn comp_max(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut max_val = None;
		let time = Duration::span(||{
	    	max_val = Some(clone.fold_up(Rc::new(|e:&E|e.clone()),Rc::new(|e1:E,e2:E|max(e1,e2))))
		});
		(time,max_val.unwrap())
	}
}

impl<E:Adapt,O:Adapt,I,B,G:Rng>
CompTreeFold<E,O,I,B> for EvalIRaz<E,G> where
	I:'static + Fn(&E)->O,
	B:'static + Fn(O,O)->O,
{
	type Target = Option<O>;
	fn comp_tfold(&self, init:Rc<I>, bin:Rc<B>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut res = None;
		let time = Duration::span(||{
	    	res = Some(clone.fold_up(init,bin))
		});
		(time,res.unwrap())
	}
}

impl<E:Adapt,O:Adapt,F,G:Rng>
CompMap<E,O,F> for EvalIRaz<E,G> where
	F:'static + Fn(&E)->O
{
	type Target = IRazTree<O>;
	fn comp_map(&self, f:Rc<F>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut mapped = None;
		let time = Duration::span(||{
			mapped = Some(clone.map(f));
		});
		(time, mapped.unwrap())
	}

}

impl<E:Adapt,O:Adapt,F,G:Rng>
CompFold<E,O,F> for EvalIRaz<E,G> where
	F:'static + Fn(O,&E)->O,
{
	type Target = O;
	fn comp_fold(&self, accum: O, f:Rc<F>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut res = None;
		let time = Duration::span(||{
			res = Some(clone.fold_lr(accum,f));
		});
		(time, res.unwrap())
	}
}

// impl<E: Adapt,O:Adapt,F,G:Rng>
// CompFold<E,O,F> for EvalIRaz<E,G> where
// 	F:'static + Fn(O,&E,&mut StdRng)->O,
// {
// 	type Target = O;
// 	fn comp_fold(&self, accum: O, f:Rc<F>, rng: &mut StdRng) -> (Duration,Self::Target) {
// 		let clone = self.raztree.clone().unwrap();
// 		let mut res = None;
// 		let time = Duration::span(||{
// 			res = Some(clone.fold_lr(accum,Rc::new(|a:O,e:&E|{f(a,e,rng)})));
// 		});
// 		(time, res.unwrap())
// 	}
// }
