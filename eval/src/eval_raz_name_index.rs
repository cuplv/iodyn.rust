//use std::fmt::{self,Debug};
use std::cmp;
use std::rc::Rc;
use rand::{Rng,StdRng,Rand};
use time::Duration;
use adapton::engine::*;
use iodyn::memo::MemoFrom;
use iodyn::raz_meta::{RazMeta,Names,FirstLast};
use iodyn::raz::{Raz, RazTree};
use iodyn::archive_stack::{AtTail};
use iodyn::tree_cursor::gen_level;
use primitives::*;
use interface::{Adapt};

/// Test harness for the incremental Raz
///
/// Coorinates elements, insertion location, gen'ed levels
#[derive(Clone,Debug)]
pub struct EvalRazNameIndex<E:Adapt,G:Rng> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<RazTree<E,Names>>,
	names: usize,
	coord: G,
	counter: usize, // for name/levels during edit
	datagauge: usize,
	namegauge: usize,
}

impl<E:Adapt,G:Rng> EvalRazNameIndex<E,G> {
	pub fn new(us: usize, ns: usize, coord:G) -> Self {
		EvalRazNameIndex {
			raztree: None,
			names: 0,
			coord: coord,
			counter: 0,
			datagauge: us,
			namegauge: ns,
		}
	}
	pub fn next_name(&mut self) -> Name {
		let n = self.names;
		self.names += 1;
		name_of_usize(n)
	}
}


impl<E:Adapt,G:Rng+Clone>
CreateEmpty<G> for EvalRazNameIndex<E,G>{
	fn inc_empty(datagauge: usize, namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = EvalRazNameIndex::new(datagauge, namegauge, (*coord).clone());
		let time = Duration::span(||{
			eval.raztree = Some(Raz::new().unfocus());
		});
		(time,eval)
	}
}

impl<E:Adapt,G:Rng+Clone>
CreateFrom<Raz<E,Names>,G> for EvalRazNameIndex<E,G>{
	fn inc_from(data: Raz<E,Names>, datagauge: usize, namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = EvalRazNameIndex::new(datagauge, namegauge, (*coord).clone());
		let time = Duration::span(||{
			eval.raztree = Some(data.unfocus());
		});
		(time,eval)
	}
}

impl<E:Adapt,G:Rng+Clone>
CreateFrom<RazTree<E,Names>,G> for EvalRazNameIndex<E,G>{
	fn inc_from(data: RazTree<E,Names>, datagauge: usize, namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = EvalRazNameIndex::new(datagauge, namegauge, (*coord).clone());
		let time = Duration::span(||{
			eval.raztree = Some(data);
		});
		(time,eval)
	}
}

impl<E:Adapt,G:Rng+Clone>
CreateFrom<AtTail<E,u32>,G> for EvalRazNameIndex<E,G>{
	fn inc_from(data: AtTail<E,u32>, datagauge: usize, namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = EvalRazNameIndex::new(datagauge, namegauge, (*coord).clone());
		let time = Duration::span(||{
			eval.raztree = Some(RazTree::memo_from(&data));
		});
		(time,eval)
	}
}

/// Creates a `RazTree` buy inserting elements, levels, and names (pregenerated)
/// into an initially unallocated `IRaz`, and then unfocusing
// uses Params::{start,namegauge,datagauge}
impl<E:Adapt+Rand,G:Rng+Clone>
CreateInc<G> for EvalRazNameIndex<E,G> {
	fn inc_init(size: usize, datagauge: usize, namegauge: usize, coord: &G, mut rng: &mut StdRng) -> (Duration,Self)
	{
		let mut eval = EvalRazNameIndex::new(datagauge, namegauge, (*coord).clone());
		// measure stuff
		let names = size/(eval.namegauge*eval.datagauge); // integer division
		let levels = size / eval.datagauge; 
		let nonames = size - (names*eval.namegauge*eval.datagauge);
		let units = nonames / eval.datagauge; // integer division
		let nounits = nonames - (units*eval.datagauge);
		// pregenerate data
		let mut lev_vec = Vec::with_capacity(levels);
		for _ in 0..levels {
			lev_vec.push(gen_level(rng))
		}
		let mut name_vec = Vec::with_capacity(names*eval.namegauge);
		for _ in 0..names {
			for _ in 0..(eval.namegauge-1){
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
			let mut raz = Raz::new();
			for _ in 0..names {
				for _ in 0..eval.namegauge {
					for _ in 0..eval.datagauge {
						raz.push_left(data_iter.next().unwrap_or_else(||panic!("init")));
					}
					raz.archive_left(level_iter.next().unwrap_or_else(||panic!("init")), name_iter.next().unwrap_or_else(||panic!("init")));
				}
				// name inserted above
			}
			for _ in 0..units {
				for _ in 0..eval.datagauge {
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
EditInsert for EvalRazNameIndex<E,G> {
	fn insert(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap_or_else(||panic!("raz uninitialized"));
		let loc = {
			let mut names = tree.meta().0.keys();
			let name_count = names.len();
			names.nth(self.coord.gen::<usize>() % name_count).unwrap().clone()
		};
		let mut focus = None;
		let focus_time = Duration::span(||{
			focus = tree.focus(loc);
		});
		let mut raz = focus.unwrap_or_else(||panic!("bad edit location"));
		// pregenerate data
		let new_levels = batch_size / self.datagauge;
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels + 1) {
			lev_vec.push(gen_level(rng))
		}
		let new_names = batch_size / (self.namegauge*self.datagauge);
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
				if self.counter % (self.namegauge*self.datagauge) == 0 {
					raz.archive_left(lev_iter.next().expect("lev_name"),name_iter.next().expect("name"));
				} else if self.counter % self.datagauge == 0 {
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
EditAppend for EvalRazNameIndex<E,G> {
	fn append(mut self, batch_size: usize, rng: &mut StdRng) -> (Duration,Self) {
		let tree = self.raztree.take().unwrap_or_else(||panic!("raz uninitialized"));
		let mut focus = None;
		let focus_time = Duration::span(||{
			focus = tree.focus(<Names as RazMeta<E>>::Index::last());
		});
		let mut raz = focus.unwrap_or_else(||panic!("bad edit location"));
		// pregenerate data
		let new_levels = batch_size / self.datagauge;
		let mut lev_vec = Vec::with_capacity(new_levels);
		for _ in 0..(new_levels + 1) {
			lev_vec.push(gen_level(rng))
		}
		let new_names = batch_size / (self.namegauge*self.datagauge);
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
				if self.counter % (self.namegauge*self.datagauge) == 0 {
					raz.archive_left(lev_iter.next().expect("lev_name"),name_iter.next().expect("name"));
				} else if self.counter % self.datagauge == 0 {
					raz.archive_left(lev_iter.next().expect("lev"),None);
				}
			}
			self.raztree = Some(raz.unfocus());
		});
		(focus_time+time,self)		
	}
}

impl<E:Adapt+Ord,G:Rng>
CompMax for EvalRazNameIndex<E,G> {
	type Target = Option<E>;
	fn comp_max(&self, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut max_val = None;
		let time = Duration::span(||{
	    	max_val = Some(clone.fold_up(Rc::new(|e:&E|e.clone()),Rc::new(|e1:E,e2:E|cmp::max(e1,e2))))
		});
		(time,max_val.unwrap())
	}
}

impl<E:Adapt,O:Adapt,I,B,G:Rng>
CompTreeFold<E,O,I,B> for EvalRazNameIndex<E,G> where
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
CompMap<E,O,F> for EvalRazNameIndex<E,G> where
	F:'static + Fn(&E)->O
{
	type Target = RazTree<O,Names>;
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
CompFold<E,O,F> for EvalRazNameIndex<E,G> where
	F:'static + Fn(O,&E)->O,
{
	type Target = O;
	fn comp_fold(&self, accum: O, f:Rc<F>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut res = None;
		let time = Duration::span(||{
			res = Some(clone.fold_lr_meta(accum,f,Rc::new(|a,_|{a})));
		});
		(time, res.unwrap())
	}
}

impl<E:Adapt,O:Adapt,F,N,G:Rng>
CompFoldMeta<E,O,(u32,Option<Name>),F,N> for EvalRazNameIndex<E,G> where
	F:'static + Fn(O,&E)->O,
	N:'static + Fn(O,(u32,Option<Name>))->O,
{
	type Target = O;
	fn comp_fold_meta(&self, accum: O, f:Rc<F>, n:Rc<N>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut res = None;
		let time = Duration::span(||{
			res = Some(clone.fold_lr_meta(accum,f,n));
		});
		(time, res.unwrap())
	}
}

impl<E:Adapt,O:Adapt,F,FF,N,G:Rng>
CompFoldArchive<E,O,u32,F,FF,N> for EvalRazNameIndex<E,G> where
	F:'static + Fn(O,&E)->O,
	FF:'static + Fn(O,Option<Name>)->O,
	N:'static + Fn(O,u32)->O,
{
	type Target = O;
	fn comp_fold_archive(&self, accum: O, f:Rc<F>, ff:Rc<FF>, n:Rc<N>, _rng: &mut StdRng) -> (Duration,Self::Target) {
		let clone = self.raztree.clone().unwrap();
		let mut res = None;
		let time = Duration::span(||{
			res = Some(clone.fold_lr_archive(accum,f,ff,n));
		});
		(time, res.unwrap())
	}
}

