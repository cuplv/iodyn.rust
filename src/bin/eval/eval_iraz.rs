use std::cmp::{min,max};
use std::rc::Rc;
use rand::StdRng;
use adapton::engine::*;
use pmfp_collections::{IRaz, IRazTree};
use pmfp_collections::inc_tree_cursor::gen_level;
use Params;
use eval::*;

pub struct EvalIRaz<'a, E:Eval,G:ElemGen<E>> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<IRazTree<E>>,
	names: usize,
	data: G,
	glob: &'a Params
}

impl<'a, E: Eval,G:ElemGen<E>> EvalIRaz<'a, E,G> {
	pub fn new(p: &'a Params, data:G) -> Self {
		EvalIRaz {
			raztree: None,
			names: 0,
			data: data,
			glob: p,
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
impl<'a, 'b, E:Eval,G:ElemGen<E>> DataInit<'a,'b,E,G> for EvalIRaz<'a, E,G> {
	fn init(p: &'a Params, data: G, mut rng: &'b mut StdRng) -> (Duration,Self)
	{
		let mut eval = EvalIRaz::new(p,data);
		let mut raz = IRaz::new();
		// measure stuff
		let names = p.start/(p.namesize*p.unitsize); // integer division
		let levels = p.start / p.unitsize; 
		let nonames = p.start - names;
		let units = nonames / p.unitsize; // integer division
		let nounits = nonames - units;
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
						raz.push_left(data_iter.next().unwrap());
					}
					raz.archive_left(level_iter.next().unwrap(), name_iter.next().unwrap());
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..p.unitsize {
					raz.push_left(data_iter.next().unwrap());
				}
				raz.archive_left(level_iter.next().unwrap(), None);
			}
			for _ in 0..nounits {
				raz.push_left(data_iter.next().unwrap());
			}
			eval.raztree = Some(raz.unfocus());
		});
		(time,eval)
	}
}

/// Appends to a `RazTree` by focusing to the end, pushing
/// data, levels, and names, then unfocusing
// uses (saved) Params::{namesize,unitsize}, EditParams::{batch_size}
impl<'a, E:Eval,G:ElemGen<E>> DataAppend for EvalIRaz<'a, E,G> {
	fn edit(mut self, p: &EditParams, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap();
		let namesize = self.glob.namesize;
		let unitsize = self.glob.unitsize;

		// measure stuff
		let mut len = tree.len();
		let mut newelems = p.batch_size;
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
		let mut data_iter = self.data.gen_count(p.batch_size,self.glob).into_iter();
		let mut name_iter = name_vec.into_iter();
		let mut level_iter = lev_vec.into_iter();

		// time the append
		let time = Duration::span(||{
			// finish the last level, name
			let mut raz = tree.focus(len).unwrap();
			for _ in 0..pre_elems {
				raz.push_left(data_iter.next().unwrap());
			}
			for _ in 0..pre_levels {
				raz.archive_left(level_iter.next().unwrap(), None);
				for _ in 0..unitsize {
					raz.push_left(data_iter.next().unwrap());
				}
			}
			if madename == 1 {
				raz.archive_left(level_iter.next().unwrap(), name_iter.next().unwrap())
			} else if madelevel == 1 {
				raz.archive_left(level_iter.next().unwrap(), None)
			}
			// add new elms, levels, names, like above
			for _ in 0..names {
				for _ in 0..namesize {
					for _ in 0..unitsize {
						raz.push_left(data_iter.next().unwrap());
					}
					raz.archive_left(level_iter.next().unwrap(), name_iter.next().unwrap());
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..unitsize {
					raz.push_left(data_iter.next().unwrap());
				}
				raz.archive_left(level_iter.next().unwrap(), None);
			}
			for _ in 0..nounits {
				raz.push_left(data_iter.next().unwrap());
			}
			self.raztree = Some(raz.unfocus());
		});
		(time,self)
	}
}

impl<'a, E:Eval+Ord,G:ElemGen<E>> DataMax<E> for EvalIRaz<'a, E,G> {
	type Target = Option<E>;
	fn compute(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut max_val = None;
		let time = Duration::span(||{
	    max_val = Some(clone.fold_up(Rc::new(|e:&E|e.clone()),Rc::new(|e1:E,e2:E|max(e1,e2))))
		});
		(time,max_val.unwrap())
	}
}

