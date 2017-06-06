//! Gauged Incremental Random Access Zipper
//!
//! RAZ - random access sequence
//!
//! - cursor access in low-const O(1) time
//! - arbitrary access in O(log n) time
//! - combines tree cursor with stack
//!
//! A Raz has two modes, one for editing(Raz), and
//! one for computations over all the data(RazTree).
//! A user calls `focus` on a RazTree to begin editing
//! mode, then `unfocus` to return to compute mode.
//!
//! Build the dataset by pushing items into the Raz, while
//! occasionally running `archive_left` (or `archive_right`)
//! to define subsequences.
//! These subsequences are used by the incremental
//! computation engine to boost performance. For simple
//! computations over the RazTree, archives every 1000 or so
//! elements work well. Consistency is not necessary, since
//! insertions and deletions are expected. `archive_left` takes a
//! level and a name. The level can be generated with the
//! crate-level function `inc_level`. Names must be unique, and
//! can be generated with `adapton::engine::*`'s `name_of_usize(num)`,
//! by passing a number from a counter.

use std::rc::Rc;

use std::fmt::Debug;
use std::hash::Hash;

use level_tree::{Tree};
use tree_cursor as tree;
use tree_cursor::TreeUpdate;
use archive_stack::{self as stack, AtTail, AtHead};
use raz_meta::{RazMeta,Navigation,FirstLast};
use memo::{MemoFrom};

use adapton::macros::*;
use adapton::engine::*;

/// Random access zipper
///
/// A cursor into a sequence
#[derive(Clone,Eq,PartialEq,Hash,Debug)]
pub struct Raz<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>+'static> {
	l_forest: tree::Cursor<TreeData<E,M>>,
	l_stack: stack::AStack<E,u32>,
	r_stack: stack::AStack<E,u32>,
	r_forest: tree::Cursor<TreeData<E,M>>,
}

const DEFAULT_SECTION_CAPACITY: usize = 500;

/// The data stored in the tree structure of the RAZ.
#[derive(PartialEq,Eq,Debug,Hash,Clone)]
enum TreeData<E:Debug+Clone+Eq+Hash, M:RazMeta<E>> {
	Dummy, // used when rebuild is certain
	Branch(M,M),
	Leaf(Rc<Vec<E>>),
}

impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
tree::TreeUpdate for TreeData<E,M> {
	#[allow(unused_variables)]
	fn rebuild(l_branch: Option<&Self>, old_data: &Self, level: u32, name: Option<Name>, r_branch: Option<&Self>) -> Self {
		match *old_data {
			TreeData::Leaf(ref vec) => TreeData::Leaf(vec.clone()),
			_ => {
				let meta = |b: Option<&TreeData<E,M>>| {
					match b {
						None => M::from_none(level, name.clone()),
						Some(&TreeData::Dummy) => unreachable!(),
						Some(&TreeData::Leaf(ref vec)) => M::from_vec(vec,level,name.clone()),
						Some(&TreeData::Branch(ref l,ref r)) => M::from_meta(l,r,level,name.clone()),
					}
				};
				TreeData::Branch(meta(l_branch),meta(r_branch))
			},
		}
	}
}

/// Tree form of a RAZ
///
/// used between refocusing, and for running global algorithms
#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub struct RazTree<E:'static+Debug+Clone+Eq+Hash, M:RazMeta<E>+'static>{
	meta: M,
	tree: Option<tree::Tree<TreeData<E,M>>>
}

fn treetop_meta<E,M>(t: Option<&tree::Tree<TreeData<E,M>>>) -> M where
	E:Debug+Clone+Eq+Hash+'static,
	M:RazMeta<E>
{
	match t {
		None => M::from_none(0,None),
		Some(t) => match t.peek() {
			TreeData::Dummy => unreachable!(),
			TreeData::Leaf(vec) => M::from_vec(&*vec,0,None),
			TreeData::Branch(l,r) => M::from_meta(&l,&r,0,None),
		}
	}
}

/// Type passed into the function call from fold_up_gauged_safe
pub enum FoldUpGauged<E,R> {
	Sub(Rc<Vec<E>>),
	Bin(R,u32,Option<Name>,R),
}

impl<E: Debug+Clone+Eq+Hash+'static, M:RazMeta<E>> RazTree<E,M> {
	/// get the meta data
	pub fn meta(&self) -> &M {&self.meta}
	pub fn is_empty(&self) -> bool {self.tree.is_none()}

	/// create a new RazTree with no data
	pub fn empty() -> Self {
		RazTree{meta: treetop_meta(None), tree: None}
	}

	/// Combine two trees left to right
	///
	/// returns None if either tree is empty.
	// TODO: Deal with bad levels.
	pub fn join(ltree: Self, level: u32, name: Option<Name>, rtree: Self) -> Option<Self> {
		let tree = match (ltree,rtree) {
			(RazTree{tree:Some(lt),..},RazTree{tree:Some(rt),..}) => {
				// if lt.level() < level && rt.level() <= level {
					bin(lt,level,name,rt)
				// } else { return None }
			},
			_ => return None
		};
		Some(RazTree{meta: treetop_meta(Some(&tree)), tree: Some(tree)})
	}

	/// Make a RazTree from a Vec
	///
	/// This tree will contain no levels or names
	/// Returns None if the Vec is empty
	pub fn from_vec(vec: Vec<E>) -> Option<Self> {
		if vec.is_empty() { return None };
		let tree = leaf(vec,None);
		Some(RazTree{meta: treetop_meta(Some(&tree)), tree: Some(tree)})
	}

	/// Runs an incremental binary function over the sequence data
	///
	/// This is calculated from data in leaves of a tree structure,
	/// so the operation must be associative. Returns None if there
	/// are no elements.
	pub fn fold_up<I,R,B>(self, init: Rc<I>, bin: Rc<B>) -> Option<R> where
		R: 'static + Eq+Clone+Hash+Debug,
		I: 'static + Fn(&E) -> R,
		B: 'static + Fn(R,R) -> R
	{
		// TODO: memo! fold_up (only first rec call is not)
		self.tree.map(|tree| {
			tree.fold_up(Rc::new(move |l,c,r|{
				match c {
					TreeData::Leaf(ref vec) => {
						let mut iter = vec.iter().map(|elm|init(elm));
						let first = iter.next().expect("leaf with empty vec");
						// eta expansion so that bin is moved into the FnMut
						// and *bin is moved into the FnOnce
						iter.fold(first, |x,y|{ (*bin)(x,y) })
					},
					_ => { match (l,r) {
						(None, None) => panic!("branch with no data"),
						(Some(r),None) | (None, Some(r)) => r,
						(Some(r1),Some(r2)) => bin(r1,r2),
					}},
				}
			}))
		})
	}

	/// Runs an incremental binary function over the sequence data
	///
	/// This is calculated from data in leaves of a tree structure,
	/// so the operation must be associative. Returns None if there
	/// are no elements.
	/// Function pointers are used to avoid issues with equality of closures.
	pub fn fold_up_safe<R,P>(
		self, init_func: fn(E,P) -> R, bin_func: fn(R,R,P)->R, params:P,
	) -> Option<R> where
		R: 'static + Eq+Clone+Hash+Debug,
		P: 'static + Eq+Clone+Hash+Debug,
	{
		fn raz_fold<E,M,R,P>(
			l:Option<R>, e:TreeData<E,M>, _lv:u32, _nm:Option<Name>, r:Option<R>,
			(init_func,bin_func,params):(fn(E,P)->R,fn(R,R,P)->R,P)
		) -> R where
			E: 'static + Eq+Clone+Hash+Debug,
			M: RazMeta<E>,
			R: 'static + Eq+Clone+Hash+Debug,
			P: 'static + Eq+Clone+Hash+Debug,
		{
			match e {
				TreeData::Leaf(vec) => {
					// TODO: remove these clones
					// the problem is that functions with references as parameters
					// do not impl Clone (but do impl Copy...)
					let mut iter = vec.iter().map(|elm|init_func((*elm).clone(),params.clone()));
					let first = iter.next().expect("leaf with empty vec");
					iter.fold(first, |x,y|bin_func(x,y,params.clone()))
				},
				_ => { match (l,r) {
					(None, None) => panic!("branch with no data"),
					(Some(r),None) | (None, Some(r)) => r,
					(Some(r1),Some(r2)) => bin_func(r1,r2,params),
				}},
			}
		}
		self.tree.map(|tree|{tree.fold_up_meta_safe(raz_fold,(init_func,bin_func,params))})
	}

	/// Runs an incremental binary function over the sequence data, levels, and names
	///
	/// This is calculated from data in leaves of a tree structure,
	/// so the operation must be associative. Returns None if there
	/// are no elements.
	pub fn fold_up_nl<I,R,B,N>(self, init: Rc<I>, bin: Rc<B>, binnl: Rc<N>) -> Option<R> where
		R: 'static + Eq+Clone+Hash+Debug,
		I: 'static + Fn(&E) -> R,
		B: 'static + Fn(R,R) -> R,
		N: 'static + Fn(R,u32,Option<Name>,R) -> R,
	{
		// TODO: memo! fold_up (only first rec call is not)
		self.tree.map(|tree| {
			tree.fold_up_meta(Rc::new(move |l,c,lv,n,r|{
				match c {
					TreeData::Leaf(ref vec) => {
						let mut iter = vec.iter().map(|elm|init(elm));
						let first = iter.next().expect("leaf with empty vec");
						// eta expansion so that bin is moved into the FnMut
						// and *bin is moved into the FnOnce
						iter.fold(first, |x,y|{ (*bin)(x,y) })
					},
					_ => { match (l,r) {
						(None, None) => panic!("branch with no data"),
						(Some(r),None) | (None, Some(r)) => r,
						(Some(lr),Some(rr)) => binnl(lr,lv,n,rr),
					}},
				}
			}))
		})
	}

	/// Runs an incremental binary function over subsequences, levels, and names
	///
	/// Subsequences allow potential for optimization. The binary function
	/// still operates on the results from the subsequence conputation.
	pub fn fold_up_gauged<I,R,B>(self, init: Rc<I>, bin: Rc<B>) -> Option<R> where
		R: 'static + Eq+Clone+Hash+Debug,
		I: 'static + Fn(&Vec<E>) -> R,
		B: 'static + Fn(R,u32,Option<Name>,R) -> R,
	{
		// TODO: memo! fold_up (only first rec call is not)
		self.tree.map(|tree| {
			tree.fold_up_meta(Rc::new(move |l,c,lv,n,r|{
				match c {
					TreeData::Leaf(ref vec) => {
						init(vec)
					},
					_ => { match (l,r) {
						(None, None) => panic!("branch with no data"),
						(Some(r),None) | (None, Some(r)) => r,
						(Some(lr),Some(rr)) => bin(lr,lv,n,rr),
					}},
				}
			}))
		})
	}

	/// Runs an incremental binary function over subsequences, levels, and names
	///
	/// Subsequences allow potential for optimization. The binary function
	/// still operates on the results from the subsequence conputation.
	/// Function pointers are used to avoid issues with equality of closures.
	pub fn fold_up_gauged_safe<R,P>(
		self, func: fn(FoldUpGauged<E,R>,P) -> R, params:P,
	) -> Option<R> where
		R: 'static + Eq+Clone+Hash+Debug,
		P: 'static + Eq+Clone+Hash+Debug,
	{
		fn raz_fold<E,M,R,P>(
			l:Option<R>, e:TreeData<E,M>, lv:u32, nm:Option<Name>, r:Option<R>,
			(func,params):(fn(FoldUpGauged<E,R>,P)->R,P)
		) -> R where
			E: 'static + Eq+Clone+Hash+Debug,
			M: RazMeta<E>,
			R: 'static + Eq+Clone+Hash+Debug,
			P: 'static + Eq+Clone+Hash+Debug,
		{
			match e {
				TreeData::Leaf(vec) => { func(FoldUpGauged::Sub(vec),params) },
				_ => { match (l,r) {
					(None, None) => panic!("branch with no data"),
					(Some(r),None) | (None, Some(r)) => r,
					(Some(lr),Some(rr)) => func(FoldUpGauged::Bin(lr,lv,nm,rr),params),
				}}
			}
		}
		self.tree.map(|tree|{tree.fold_up_meta_safe(raz_fold,(func,params))})
	}

	/// Runs an incremental fold over the sequence, left to right
	pub fn fold_lr<A,B>(self, init: A, bin: Rc<B>) -> A where
		A: 'static + Eq+Clone+Hash+Debug,
		B: 'static + Fn(A,&E) -> A,
	{ self.fold_lr_meta(init,bin,Rc::new(|a,_|{a})) }

	/// left-to-right memoized fold with levels and names
	pub fn fold_lr_meta<A,B,N>(self, init: A, bin: Rc<B>, meta: Rc<N>) -> A where
		A: 'static + Eq+Clone+Hash+Debug,
		B: 'static + Fn(A,&E) -> A,
		N: 'static + Fn(A,(u32,Option<Name>)) -> A,
	{
		let start_name = Some(name_of_string(String::from("start")));
		match self.tree {
			None => init,
			Some(tree) => {
				tree.fold_lr_meta(start_name,init,Rc::new(move|a,e,l,n|{
					match e {
						TreeData::Leaf(ref vec) => {
							vec.iter().fold(a,|a,e|{bin(a,e)})
						},
						_ => {
							meta(a,(l,n))
						},
					}
				}))
			},
		}
	}

	/// left-to-right incremental fold with levels and names, with a name provided at the leaf
	pub fn fold_lr_archive<A,B,F,N>(self, init: A, bin: Rc<B>, finbin: Rc<F>, meta: Rc<N>) -> A where
		A: 'static + Eq+Clone+Hash+Debug,
		B: 'static + Fn(A,&E) -> A,
		F: 'static + Fn(A,Option<Name>) -> A,
		N: 'static + Fn(A,u32) -> A,
	{
		let start_name = Some(name_of_string(String::from("start")));
		match self.tree {
			None => init,
			Some(tree) => {
				tree.fold_lr_meta(start_name,init,Rc::new(move|a,e,l,n|{
					match e {
						TreeData::Leaf(ref vec) => {
								finbin(vec.iter().fold(a,|a,e|{bin(a,e)}),n)
						},
						_ => {
							meta(a,l)
						},
					}
				}))
			},
		}
	}

	/// An incremental mapping of the tree, returning a new tree
	pub fn map<R,F,N:RazMeta<R>>(self, f: Rc<F>) -> RazTree<R,N> where
		R: 'static + Eq+Clone+Hash+Debug,
		F: 'static + Fn(&E) -> R,
	{
		// TODO: memo! map (only the first rec call is not)
		let tree = self.tree.map(|tree| {
			tree.map(Rc::new(move |
				d: TreeData<E,M>,
				lev,n,
				l:Option<&Tree<TreeData<R,N>>>,
				r:Option<&Tree<TreeData<R,N>>>,
			|{
				match d {
					TreeData::Leaf(ref vec) => {
						let mapped = vec.iter().map(|e|f(e)).collect();
						TreeData::Leaf(Rc::new(mapped))
					},
					_ => {
						TreeData::rebuild(
							l.map(|t|t.peek()).as_ref(),
							&TreeData::Dummy,
							lev, n,
							r.map(|t|t.peek()).as_ref(),
						)
					},
				}
			}))
		});
		RazTree{meta: treetop_meta(tree.as_ref()), tree: tree}
	}


	/// focus on a location in the sequence to begin editing.
	pub fn focus<I:Into<M::Index>>(self, index: I) -> Option<Raz<E,M>> {
		let mut index = index.into();
		match self { 
			RazTree{tree:None, ..} => {
				Some(Raz{
					l_forest: tree::Cursor::new(),
					l_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
					r_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
					r_forest: tree::Cursor::new(),
				})
			},
			RazTree{tree: Some(tree), ..} => {
				// step 1: find location with cursor
				let mut cursor = tree::Cursor::from(tree);
				while let TreeData::Branch(l,r) = cursor.peek().unwrap() {
					match M::navigate(&l,&r,&index) {
						Navigation::Left(i) => {
							assert!(cursor.down_left());
							index = i;
						},
						Navigation::Right(i) => {
							assert!(cursor.down_right());
							index = i;
						},
						Navigation::Here => {
							assert!(cursor.down_left());
							while cursor.down_right() {}
							index = M::Index::last();
						},
						Navigation::Nowhere => { return None }, 
					}
				}
				// step 2: extract and copy data
				let mut l_astack = stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY);
				let mut r_astack = stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY);
				let (l_cursor, tree, r_cursor) = cursor.split();
				match tree {
					Some(ref t) => match t.peek() {
						TreeData::Leaf(ref vec_ref) => {
							let (l_slice,r_slice) = M::split_vec(vec_ref, &index);
							l_astack.extend(l_slice);
							r_astack.extend_rev(r_slice);
						},
						_ => unreachable!(),
					},
					None => unreachable!(),
				};
				// step 3: integrate
				Some(Raz{
					l_forest: l_cursor,
					l_stack: l_astack,
					r_stack: r_astack,
					r_forest: r_cursor,
				})
			},
		}
	}

	/// focus on the first element in the sequence
	pub fn focus_left(self) -> Raz<E,M> {
		match self { 
			RazTree{tree:None, ..} => {
				Raz{
					l_forest: tree::Cursor::new(),
					l_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
					r_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
					r_forest: tree::Cursor::new(),
				}
			},
			RazTree{tree: Some(tree), ..} => {
				// step 1: find location with cursor
				let mut cursor = tree::Cursor::from(tree);
				while cursor.down_left() {}
				// step 2: extract and copy data
				let l_astack = stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY);
				let mut r_astack = stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY);
				let (l_cursor, tree, r_cursor) = cursor.split();
				match tree {
					Some(ref t) => match t.peek() {
						TreeData::Leaf(ref vec_ref) => {
							r_astack.extend_rev(vec_ref);
						},
						_ => unreachable!(),
					},
					None => unreachable!(),
				};
				// step 3: integrate
				Raz{
					l_forest: l_cursor,
					l_stack: l_astack,
					r_stack: r_astack,
					r_forest: r_cursor,
				}
			},
		}
	}

	pub fn into_iter_lr(self) -> IterR<E,M> {
		let Raz{r_stack, r_forest, ..} = self.focus_left();
		let mut current = r_stack.active_data().clone();
		current.reverse();
		let (_,_,iter) = r_forest.into_iters();
		IterR {
			items: current.into_iter(),
			cursor: iter,
		}
	}

	pub fn into_iter_rl(self) -> IterL<E,M> {
		let Raz{l_stack, l_forest, ..} = self.focus(M::Index::last()).unwrap();
		let mut current = l_stack.active_data().clone();
		let (iter,_,_) = l_forest.into_iters();
		current.reverse();
		IterL {
			items: current.into_iter(),
			cursor: iter,
		}
	}

}

impl<T: Debug+Clone+Eq+Hash+'static, M:RazMeta<T>>
IntoIterator for RazTree<T,M> {
	type Item = T;
	type IntoIter = IterR<T,M>;
	fn into_iter(self) -> Self::IntoIter {
		self.into_iter_lr()
	}
}

impl<E: Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
Raz<E,M> {
	/// Create a new RAZ, for an empty sequence
	pub fn new() -> Raz<E,M> {
		Raz{
			l_forest: tree::Cursor::new(),
			l_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
			r_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
			r_forest: tree::Cursor::new(),
		}
	}

	/// unfocus the Raz, switching to compute mode
	pub fn unfocus(self) -> RazTree<E,M> { self.choose_unfocus(false) }
	/// memoized unfocus
	pub fn memo_unfocus(self) -> RazTree<E,M> { self.choose_unfocus(true) }
	/// unfocus with parametrized choice to memoize
	pub fn choose_unfocus(self,memo:bool) -> RazTree<E,M> {
		// helper functions for building trees from sequences
		let tree_name = name_of_str("tree");
		let tailtree = |stack: stack::AStack<E,u32>| {
			ns(tree_name.clone(),||{
				tree::Cursor::from(tree_of_tailstack(&stack,memo).unwrap())
			})
		};
		let tailtree_seed = |stack: stack::AStack<E,u32>,lev,nm,sub| {
			ns(tree_name.clone(),||{
				let (_,_,_,t) = memo_build_left(stack,lev,nm,sub,u32::max_value(),memo);
				tree::Cursor::from(t)
			})
		};
		let headtree = |stack: stack::AStack<E,u32>| {
			ns(tree_name.clone(),||{
				tree::Cursor::from(tree_of_headstack(&stack,memo).unwrap())
			})
		};
		let headtree_seed = |stack: stack::AStack<E,u32>,lev,nm,sub| {
			ns(tree_name.clone(),||{
				let (_,_,_,t) = memo_build_right(stack,lev,nm,sub,u32::max_value(),memo);
				tree::Cursor::from(t)
			})
		};
		// deconstruct self
		match self { Raz{ mut l_forest, mut l_stack, mut r_stack, mut r_forest} => {

		// step 1: create center tree
		let center = match (l_stack.is_empty(), r_stack.is_empty()) {
			// possible empty stack (use the other)
			(true,true) => None,
			(false,true) => { Some(tailtree(l_stack)) },
			(true,false) => { Some(headtree(r_stack)) },
			(false,false) => { 
				// possible missing local vec (extract level,name)
				let (lc,lev,nm,rc) = match (l_stack.active_len(),r_stack.active_len()) {
					(_,0) => { 
						let nm = r_stack.name();
						let lev = r_stack.next_archive().unwrap().1.unwrap();
						let lc = Some(tailtree(l_stack));
						let rc = headtree(r_stack);
						(lc,lev,nm,rc)
					},
					(0,_) => { 
						let nm = l_stack.name();
						let lev = l_stack.next_archive().unwrap().1.unwrap();
						let lc = Some(tailtree(l_stack));
						let rc = headtree(r_stack);
						(lc,lev,nm,rc)
					},
					_ => {
						let l_nm = l_stack.name();
						let r_nm = r_stack.name();
						let (mut l_vec,l_lev) = l_stack.next_archive().unwrap();
						let (mut r_vec,r_lev) = r_stack.next_archive().unwrap();
						match (l_lev,r_lev) {
							// local vecs (join with other side)
							(None,None) => {
								r_vec.reverse();
								l_vec.extend(r_vec);
								let tree = leaf(l_vec,None);
								(None,0,None,tree.into())
							},
							(None,Some(r_lev)) => {
								l_vec.reverse();
								r_vec.extend(l_vec);
								let rc = headtree_seed(r_stack,r_lev,r_nm,leaf(r_vec,None));
								(None,0,None,rc)
							},
							(Some(l_lev),None) => {
								r_vec.reverse();
								l_vec.extend(r_vec);
								let rc = tailtree_seed(l_stack,l_lev,l_nm,leaf(l_vec,None));
								(None,0,None,rc)
							},
							(Some(l_lev),Some(r_lev)) => {
								r_vec.reverse();
								l_vec.extend(r_vec);
								let lc = Some(tailtree_seed(l_stack,l_lev,l_nm,leaf(l_vec,None)));
								(lc,r_lev,r_nm,headtree(r_stack))
							},
						}
					},
				};
				if let Some(lc) = lc {
					Some(ns(name_of_str("center_tree_join"),||{
						let mut center = tree::Cursor::join(lc,lev,nm,TreeData::Dummy,rc);
						while center.up() != tree::UpResult::Fail {}
						center
					}))
				} else {
					Some(rc)
				}
			},
		};
		// step 2: join with forests
		let left_side = if l_forest.up_discard() != tree::UpResult::Fail {
			let lev = l_forest.peek_level().unwrap();
			let nm = l_forest.peek_name();
			l_forest.down_left_force(tree::Force::Discard);
			if let Some(center) = center {
				Some(ns(name_of_str("left_forest_join"),||{
					let mut left = tree::Cursor::join(l_forest,lev,nm,TreeData::Dummy,center);
					while left.up() != tree::UpResult::Fail {}
					left
				}))
			} else {
				Some(l_forest)
			}
		} else {
			center
		};
		let complete = if r_forest.up_discard() != tree::UpResult::Fail {
			let lev = r_forest.peek_level().unwrap();
			let nm = r_forest.peek_name();
			r_forest.down_right_force(tree::Force::Discard);
			ns(name_of_str("right_forest_join"),move||{
				let mut tree = if let Some(left_side) = left_side {
					tree::Cursor::join(left_side,lev,nm,TreeData::Dummy,r_forest)
				} else {
					r_forest
				};
				while tree.up() != tree::UpResult::Fail {}
				Some(tree)
			})
		} else {
			left_side
		};
		// step 3: convert to final tree
		if let Some(complete) = complete {
			let tree = complete.at_tree();
			RazTree{meta: treetop_meta(tree.as_ref()), tree: tree}
		} else {
			RazTree::empty()
		}

		}} // end deconstruct self
	}
  
 //  /// creates two iterators, one for each side of the cursor
	// pub fn into_iters(self) -> (IterL<E>,IterR<E>) {
	// 	match self {
	// 		Raz{
	// 			l_forest,
	// 			l_stack,
	// 			r_stack,
	// 			r_forest,
	// 		} =>
	// 		(IterL(Raz{
	// 			l_forest: l_forest,
	// 			l_stack: l_stack,
	// 			r_stack: stack::AStack::new(),
	// 			r_forest: tree::Cursor::new(),
	// 		}),
	// 		IterR(Raz{
	// 			l_forest: tree::Cursor::new(),
	// 			l_stack: stack::AStack::new(),
	// 			r_stack: r_stack,
	// 			r_forest: r_forest,
	// 		}))
	// 	} 
	// }

	/// add an element to the left of the cursor
	///
	/// returns number of non-archived elements
	pub fn push_left(&mut self, elm: E) -> usize {
		self.l_stack.push(elm);
		self.l_stack.active_len()
	}
	/// add an element to the right of the cursor
	///
	/// returns number of non-archived elements
	pub fn push_right(&mut self, elm: E) -> usize {
		self.r_stack.push(elm);
		self.r_stack.active_len()
	}
	/// peek at the element to the left of the cursor
	pub fn peek_left(&self) -> Option<E> {
		if self.l_stack.is_empty() {
			let mut peek_forest = self.l_forest.clone();
			if peek_forest.up() == tree::UpResult::Fail { return None } else {
				peek_forest.down_left();
				while peek_forest.down_right() {}
				match peek_forest.peek() {
					Some(TreeData::Leaf(ref data)) => data.last().map(|e|e.clone()),
					_ => panic!("peek_left: no left tree leaf"),
				}
			}
		} else { self.l_stack.peek() }
	}
	/// peek at the element to the left of the cursor
	pub fn peek_right(&self) -> Option<E> {
		if self.r_stack.is_empty() {
			let mut peek_forest = self.r_forest.clone();
			if peek_forest.up() == tree::UpResult::Fail { return None } else {
				peek_forest.down_right();
				while peek_forest.down_left() {}
				match peek_forest.peek() {
					Some(TreeData::Leaf(ref data)) => data.first().map(|e|e.clone()),
					_ => panic!("peek_right: no right tree leaf"),
				}
			}
		} else { self.r_stack.peek() }
	}
	/// mark the data at the left to be part of a subsequence
	///
	/// Levels determine internal structure. Most usages should
	/// call `iodyn::inc_level()` to generate one.
	pub fn archive_left(&mut self, level: u32, name: Option<Name>) {
		let level = if level == 0 { 1 } else { level };
		ns(name_of_str("zip"),move||{self.l_stack.archive(name,level)});
	}
	/// mark the data at the right to be part of a subsequence
	///
	/// Levels determine internal structure. Most usages should
	/// call `iodyn::inc_level()` to generate one.
	pub fn archive_right(&mut self, level: u32, name: Option<Name>) {
		let level = if level == 0 { 1 } else { level };
		ns(name_of_str("zip"),move||{self.r_stack.archive(name,level)});
	}

	/// remove and return an element to the left of the cursor
	///
	/// if an archive point is to the left of the cursor,
	/// it will be removed
	pub fn pop_left(&mut self) -> Option<E> {
		if self.l_stack.is_empty() {
			if self.l_forest.up_discard() == tree::UpResult::Fail { return None } else {
				self.l_forest.down_left_force(tree::Force::Discard);
				while self.l_forest.down_right() {}
				match self.l_forest.peek() {
					Some(TreeData::Leaf(ref data)) => self.l_stack.extend(&***data),
					_ => panic!("pop_left: no left tree leaf"),
				}
			}
		}
		self.l_stack.pop()
	}
	/// remove and return an element to the right of the cursor
	///
	/// if an archive point is to the right of the cursor,
	/// it will be removed
	pub fn pop_right(&mut self) -> Option<E> {
		if self.r_stack.is_empty() {
			if self.r_forest.up_discard() == tree::UpResult::Fail { return None } else {
				self.r_forest.down_right_force(tree::Force::Discard);
				while self.r_forest.down_left() {}
				match self.r_forest.peek() {
					Some(TreeData::Leaf(ref data)) => self.r_stack.extend_rev(&***data),
					_ => panic!("pop_right: no right tree leaf"),
				}
			}
		}
		self.r_stack.pop()
	}

	/// remove and return an element to the left of the cursor
	/// along with any level or name that was consumed 
	///
	/// if an archive point is to the left of the cursor,
	/// it will be removed and the level returned
	/// if there was also a name present it will be returned
	pub fn pop_left_level_name(&mut self) -> Option<(E,Option<(u32,Option<Name>)>)> {
		if self.l_stack.is_empty() {
			if self.l_forest.up_discard() == tree::UpResult::Fail { return None } else {
				let lev = self.l_forest.peek_level().unwrap();
				let nm = self.l_forest.peek_name();
				self.l_forest.down_left_force(tree::Force::Discard);
				while self.l_forest.down_right() {}
				match self.l_forest.peek() {
					Some(TreeData::Leaf(ref data)) => self.l_stack.extend(&***data),
					_ => panic!("pop_left_level_name: no left tree leaf"),
				}
				Some((self.l_stack.pop().unwrap(), Some((lev, nm))))
			}
		} else { self.l_stack.pop().map(|val|{ (val,None) }) }
	}
	/// remove and return an element to the right of the cursor
	/// along with any level or name that was consumed 
	///
	/// if an archive point is to the right of the cursor,
	/// it will be removed and the level returned
	/// if there was also a name present it will be returned
	pub fn pop_right_level_name(&mut self) -> Option<(E,Option<(u32,Option<Name>)>)> {
		if self.r_stack.is_empty() {
			if self.r_forest.up_discard() == tree::UpResult::Fail { return None } else {
				let lev = self.r_forest.peek_level().unwrap();
				let nm = self.r_forest.peek_name();
				self.r_forest.down_right_force(tree::Force::Discard);
				while self.r_forest.down_left() {}
				match self.r_forest.peek() {
					Some(TreeData::Leaf(ref data)) => self.r_stack.extend_rev(&***data),
					_ => panic!("pop_right_level_name: no right tree leaf"),
				}
				Some((self.r_stack.pop().unwrap(), Some((lev, nm))))
			}
		} else { self.r_stack.pop().map(|val|{ (val,None) }) }
	}
}

pub struct IterR<T: Debug+Clone+Eq+Hash+'static, M:RazMeta<T>+'static>{
	items: ::std::vec::IntoIter<T>,
	cursor: tree::IterR<TreeData<T,M>>,
}
impl<T: Debug+Clone+Eq+Hash+'static, M:RazMeta<T>>
Iterator for IterR<T,M> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		if let Some(val) = self.items.next() { return Some(val) }
		match self.cursor.next() {
			None => return None,
			Some(TreeData::Dummy) => unreachable!(),
			Some(TreeData::Leaf(vec)) => { self.items = (*vec).clone().into_iter() },
			Some(TreeData::Branch{..}) => {},
		}
		self.next()
	}
}

pub struct IterL<T: Debug+Clone+Eq+Hash+'static, M:RazMeta<T>+'static>{
	items: ::std::vec::IntoIter<T>,
	cursor: tree::IterL<TreeData<T,M>>,
}
impl<T: Debug+Clone+Eq+Hash+'static, M:RazMeta<T>>
Iterator for IterL<T,M> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		if let Some(val) = self.items.next() { return Some(val) }
		match self.cursor.next() {
			None => return None,
			Some(TreeData::Dummy) => unreachable!(),
			Some(TreeData::Leaf(vec)) => {
				let mut items = (*vec).clone();
				items.reverse();
				self.items = items.into_iter();
			},
			Some(TreeData::Branch{..}) => {},
		}
		self.next()
	}
}

// impl<T: Debug+Clone+Eq+Hash+'static> IterR<T> {
// 	pub fn inc_fold_out<R,B>(self, init:R, bin:Rc<B>) -> R where
// 		R: 'static + Eq+Clone+Hash+Debug,
// 		B: 'static + Fn(R,&T) -> R
// 	{
// 		match self.0 {Raz{r_stack,mut r_forest, ..}=>{
// 			let stack_result = r_stack.into_iter().fold(init, |r,t|{bin(r,&t)});
// 			if r_forest.up_discard() == tree::UpResult::Fail { return stack_result }
// 			let (_,_,iter) = r_forest.into_iters();
// 			iter.fold_out(stack_result,Rc::new(move|r,t|{
// 				match t {
// 					TreeData::Branch{..} => r,
// 					TreeData::Leaf(vec) => {
// 						vec.iter().fold(r,|r,e|{bin(r,e)})
// 					},
// 				}
// 			}))
// 		}}
// 	}
// }
	

/////////////////////////////
// Traits for Raz and RazTree
/////////////////////////////
use level_tree as ltree;
use std::convert::From;

/// convenience fn for making a tree from data
#[allow(unused)]
fn leaf<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(v:Vec<E>, n: Option<Name>) -> ltree::Tree<TreeData<E,M>> {
	ltree::Tree::new(0,n,TreeData::Leaf(Rc::new(v)),None,None).unwrap()
}
/// convenience fn for combining two trees as branches
#[allow(unused)]
fn bin<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
	t1: ltree::Tree<TreeData<E,M>>,
	l:  u32,
	n:  Option<Name>,
	t2: ltree::Tree<TreeData<E,M>>
) -> ltree::Tree<TreeData<E,M>> {
	let td = TreeData::rebuild(
		Some(&t1.peek()),
		&TreeData::Dummy,
		l, n.clone(),
		Some(&t2.peek()),
	);
	ltree::Tree::new(
		l,n,td,
		Some(t1), Some(t2),
	).unwrap()
}

// joins the subtree with the stack, creating a larger tree
// subtree is rightmost subbranch of output tree
// assumes the stack has some data, and levels are appropriate
// fully consumes the stack
fn memo_build_left<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
	s:stack::AStack<E,u32>, l:u32, n:Option<Name>, t: Tree<TreeData<E,M>>, m:u32,
	memo: bool,
) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
	if !memo { return build_left(s,l,n,t,m,memo) }
	match n.clone() {
		None => return build_left(s,l,n,t,m,memo),
		Some(nm) => {
			let (nm,_) = name_fork(nm);
			return memo!(nm =>> build_left, s:s, l:l, n:n, t:t, m:m ;; memo:memo)
		},
	}
	fn build_left<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
		mut stack: stack::AStack<E,u32>,
		first_level: u32,
		first_name: Option<Name>,
		sub_tree: Tree<TreeData<E,M>>,
		max_level: u32,
		memo: bool,
	) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
		assert!(sub_tree.level() <= first_level);
		assert!(first_level < max_level);
		let next_name = stack.name();
		let (vec,next_level) = stack.next_archive().unwrap_or_else(||{panic!("stack was unexpectedly empty")});
		let leaf_tree = leaf(vec,None);
		let (shorter_stack, final_level,final_name,small_tree) = match next_level {
			None =>
				(stack::AStack::new(),None,None,leaf_tree),
			Some(lev) => if lev < first_level {
				memo_build_left(stack,lev,next_name,leaf_tree,first_level,memo)
			} else {
				(stack,Some(lev),next_name,leaf_tree)
			},
		};
		let new_tree = bin(small_tree, first_level, first_name, sub_tree);
		match final_level {
			None =>
				(stack::AStack::new(), None, None, new_tree),
			Some(lev) => if lev < max_level {
				memo_build_left(shorter_stack,lev,final_name,new_tree,max_level,memo)
			} else {
				(shorter_stack,Some(lev),final_name,new_tree)
			},
		}
	}
}

// we build this tree right to left
// note that the left branch _cannot_ have
// the same level as its parent
// while the right branch can.
fn tree_of_tailstack<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>(tailstack: &stack::AStack<E,u32>,memo:bool) -> Option<Tree<TreeData<E,M>>> {
	let mut tailstack = tailstack.clone();
	let name = tailstack.name();
	let (level, first_tree) = match tailstack.next_archive_force() {
		None => return None,
		Some((vec,None)) => {
			return Some(leaf(vec,None));
		},
		Some((vec,Some(level))) => (level,leaf(vec,None))
	};
	let (s,l,n,t) = memo_build_left(tailstack, level, name, first_tree, u32::max_value(), memo);
	assert!(l.is_none());
	assert!(n.is_none());
	assert!(s.is_empty());
	Some(t)
}
impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
MemoFrom<stack::AtTail<E,u32>>
for RazTree<E,M> {
	fn memo_from(&AtTail(ref tailstack): &stack::AtTail<E,u32>) -> Self {
		if let Some(tree) = tree_of_tailstack(tailstack,true) {
			RazTree{meta: treetop_meta(Some(&tree)), tree: Some(tree)}
		} else { RazTree::empty() }
	}
}
impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
From<stack::AtTail<E,u32>>
for RazTree<E,M> {
	fn from(AtTail(ref tailstack): stack::AtTail<E,u32>) -> Self {
		if let Some(tree) = tree_of_tailstack(tailstack,false) {
			RazTree{meta: treetop_meta(Some(&tree)), tree: Some(tree)}
		} else { RazTree::empty() }
	}
}

// joins the subtree with the stack, creating a larger tree
// subtree is rightmost subbranch of output tree
// assumes the stack has some data, and levels are appropriate
// fully consumes the stack
fn memo_build_right<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
	s:stack::AStack<E,u32>, l:u32, n:Option<Name>, t: Tree<TreeData<E,M>>, m:u32,
	memo:bool,
) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
	if !memo { return build_right(s,l,n,t,m,memo) }
	match n.clone() {
		None => return build_right(s,l,n,t,m,memo),
		Some(nm) => {
			let (nm,_) = name_fork(nm);
			return memo!(nm =>> build_right, s:s, l:l, n:n, t:t, m:m ;; memo:memo)
		},
	}
	fn build_right<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
		mut stack: stack::AStack<E,u32>,
		first_level: u32,
		first_name: Option<Name>,
		sub_tree: Tree<TreeData<E,M>>,
		max_level: u32,
		memo: bool,
	) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
		assert!(sub_tree.level() < first_level);
		assert!(first_level <= max_level);
		let next_name = stack.name();
		let (mut vec,next_level) = stack.next_archive().unwrap_or_else(||{panic!("stack was unexpectedly empty")});
		vec.reverse();
		let leaf_tree = leaf(vec,None);
		let (shorter_stack, final_level,final_name,small_tree) = match next_level {
			None =>
				(stack::AStack::new(),None,None,leaf_tree),
			Some(lev) => if lev <= first_level {
				memo_build_right(stack,lev,next_name,leaf_tree,first_level,memo)
			} else {
				(stack,Some(lev),next_name,leaf_tree)
			},
		};
		let new_tree = bin(sub_tree, first_level, first_name, small_tree);
		match final_level {
			None =>
				(stack::AStack::new(), None, None, new_tree),
			Some(lev) => if lev <= max_level {
				memo_build_right(shorter_stack,lev,final_name,new_tree,max_level,memo)
			} else {
				(shorter_stack,Some(lev),final_name,new_tree)
			},
		}
	}
}
// we build this tree left to right
// note that the left branch _cannot_ have
// the same level as its parent
// while the right branch can.
//
// vecs from the data are reversed, because Vec pushes
// to the tail, but this stack was used as if pushing
// to head. The head of a Raz in on the left.
fn tree_of_headstack<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>(headstack: &stack::AStack<E,u32>,memo:bool) -> Option<Tree<TreeData<E,M>>> {
	let mut headstack = headstack.clone();
	let name = headstack.name();
	let (level, first_tree) = match headstack.next_archive_force() {
		None => return None,
		Some((mut vec,None)) => {
			vec.reverse();
			return Some(leaf(vec,None));
		},
		Some((mut vec,Some(level))) => {
			vec.reverse();
			(level,leaf(vec,None))
		}
	};
	let (s,l,n,t) = memo_build_right(headstack, level, name, first_tree, u32::max_value(), memo);
	assert!(l.is_none());
	assert!(n.is_none());
	assert!(s.is_empty());
	Some(t)
}
impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
MemoFrom<stack::AtHead<E,u32>>
for RazTree<E,M> {
	fn memo_from(&AtHead(ref headstack): &stack::AtHead<E,u32>) -> Self {
		if let Some(tree) = tree_of_headstack(headstack,true) {
			RazTree{meta: treetop_meta(Some(&tree)), tree: Some(tree)}
		} else { RazTree::empty() }
	}
}
impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
From<stack::AtHead<E,u32>>
for RazTree<E,M> {
	fn from(AtHead(ref headstack): stack::AtHead<E,u32>) -> Self {
		if let Some(tree) = tree_of_headstack(headstack,false) {
			RazTree{meta: treetop_meta(Some(&tree)), tree: Some(tree)}
		} else { RazTree::empty() }
	}
}

////////////////////////////
// Tests for Incremental RAZ
////////////////////////////


#[cfg(test)]
mod tests {
	use super::*;
	use raz_meta::Count;
	use level_tree::good_levels;

	fn example_tree() -> RazTree<usize,Count> {
		let a = leaf(vec!(1,2),None);
		let b = leaf(vec!(3,4),None);
		let c = leaf(vec!(5,6),None);
		let d = leaf(vec!(7,8),None);
		let e = leaf(vec!(9,10),None);
		let f = leaf(vec!(11,12),None);

		let one = bin(b,1,Some(name_of_usize(1)),c);
		let two = bin(one,2,Some(name_of_usize(2)),d);
		let three = bin(a,3,Some(name_of_usize(3)),two);
		let four = bin(e,4,Some(name_of_usize(4)),f);
		let five = bin(three,5,Some(name_of_usize(5)),four);

		RazTree{
			meta: treetop_meta(Some(&five)),
			tree: Some(five),
		}
	}

  #[test]
  fn test_push_pop() {
  	let mut raz:Raz<_,()> = Raz::new();
  	raz.push_left(5);
  	raz.push_left(4);
  	raz.push_right(8);
  	raz.pop_left();

  	assert_eq!(Some(8), raz.pop_right());
  	assert_eq!(Some(5), raz.pop_left());
  	assert_eq!(None, raz.pop_right());
  }

  #[test]
  fn test_tree_focus() {
  	let tree = example_tree();
  	assert!(good_levels(tree.tree.as_ref().unwrap()));

  	let mut left = tree.clone().focus(0usize).unwrap();
  	let mut deep = tree.clone().focus(5usize).unwrap();
  	let mut right = tree.clone().focus(12usize).unwrap();

  	assert_eq!(Some(1), left.pop_right());
  	assert_eq!(Some(12), right.pop_left());
  	assert_eq!(None, right.pop_right());
  	assert_eq!(Some(5), deep.pop_left());

  	assert_eq!(Some(6), deep.pop_right());
  	assert_eq!(Some(7), deep.pop_right());
  	assert_eq!(Some(8), deep.pop_right());
  	assert_eq!(Some(9), deep.pop_right());
  	assert_eq!(Some(10), deep.pop_right());
  	assert_eq!(Some(11), deep.pop_right());
  	assert_eq!(Some(12), deep.pop_right());
  	assert_eq!(None, deep.pop_right());

  	assert_eq!(Some(11), right.pop_left());
  	assert_eq!(Some(10), right.pop_left());

  	assert_eq!(Some(4), deep.pop_left());
  	assert_eq!(Some(3), deep.pop_left());
  	assert_eq!(Some(2), deep.pop_left());
  	assert_eq!(Some(1), deep.pop_left());
  	assert_eq!(None, deep.pop_left());

  }

  #[test]
  fn test_unfocus() {
  	let mut r: Raz<_,Count> = Raz::new();
  	let mut t;
  	// set same tree as focus example
  	r.push_left(3);
  	r.push_left(4);
  	r.archive_left(1, Some(name_of_usize(1)));
  	r.push_right(8);
  	r.push_right(7);
  	r.archive_right(2, Some(name_of_usize(2)));
  	r.push_left(5);
  	r.push_right(6);
  	t = r.unfocus();
  	r = t.focus(0usize).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();

  	r = t.focus(8usize).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();

  	assert!(good_levels(t.tree.as_ref().unwrap()));

  	// focus and read
  	r = t.focus(7usize).expect("focus on 7");
  	assert_eq!(Some(7), r.pop_left());
  	assert_eq!(Some(6), r.pop_left());
  	assert_eq!(Some(5), r.pop_left());
  	assert_eq!(Some(4), r.pop_left());
  	assert_eq!(Some(3), r.pop_left());
  	assert_eq!(Some(2), r.pop_left());
  	assert_eq!(Some(1), r.pop_left());
  	t = r.unfocus();
  	r = t.focus(5usize).expect("focus on 5");
  	assert_eq!(None, r.pop_right());
  	assert_eq!(Some(12), r.pop_left());
  	assert_eq!(Some(11), r.pop_left());
  	assert_eq!(Some(10), r.pop_left());
  	assert_eq!(Some(9), r.pop_left());
  	assert_eq!(Some(8), r.pop_left());
  	assert_eq!(None, r.pop_left());
  }

  #[test]
  fn test_fold_up() {
  	let tree = example_tree();
  	assert!(good_levels(tree.tree.as_ref().unwrap()));

  	let max = tree.clone().fold_up(Rc::new(|e:&usize|{*e}),Rc::new(|e1:usize,e2:usize|{::std::cmp::max(e1,e2)})).unwrap();
  	assert_eq!(12, max);

  	let sum = tree.clone().fold_up(Rc::new(|e:&usize|{*e}),Rc::new(|e1:usize,e2:usize|{e1+e2})).unwrap_or(0);
  	let iter_sum: usize = (1..13).sum();
  	assert_eq!(iter_sum, sum);

  	#[derive(PartialEq,Eq,Debug,Hash,Clone)]
  	enum EO {Even,Odd}
  	let even_odd = tree.clone().fold_up(
  		Rc::new(|e:&usize| if *e % 2 == 0 {EO::Even} else {EO::Odd}),
  		Rc::new(|e1:EO,e2:EO| if e1 == e2 {EO::Even} else {EO::Odd})
  	).unwrap();
  	assert_eq!(EO::Even, even_odd);

  }

  #[test]
  fn test_map() {
  	let tree = example_tree();

  	let plus1: RazTree<_,Count> = tree.map(Rc::new(|e: &usize|*e+1));
  	let sum = plus1.clone().fold_up(Rc::new(|e:&usize|*e),Rc::new(|e1:usize,e2:usize|e1+e2)).unwrap_or(0);
  	let iter_sum: usize = (2..14).sum();
  	assert_eq!(iter_sum, sum);

  	// check the structure
  	let mut cursor = tree::Cursor::from(plus1.tree.unwrap());
  	assert!(cursor.down_left());
  	assert!(cursor.down_left());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![2,3], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.down_right());
  	assert!(cursor.down_left());
  	assert!(cursor.down_left());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![4,5], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  }

  #[test]
  fn test_from_tail_stack() {
  	let mut stack = stack::AStack::new();
  	stack.push(1);
  	stack.push(2);
  	stack.archive(Some(name_of_usize(3)),3);
  	stack.push(3);
  	stack.push(4);
  	stack.archive(Some(name_of_usize(1)),1);
  	stack.push(5);
  	stack.push(6);
  	stack.archive(Some(name_of_usize(2)),2);
  	stack.push(7);
  	stack.push(8);
  	stack.archive(Some(name_of_usize(5)),5);
  	stack.push(9);
  	stack.push(10);
  	stack.archive(Some(name_of_usize(4)),4);
  	stack.push(11);
  	stack.push(12);
  	let raz: RazTree<_,Count> = RazTree::memo_from(&stack::AtTail(stack));

  	// check that levels are high-to-low
  	assert!(good_levels(raz.tree.as_ref().unwrap()));
  	
  	// check that all elements are represented
  	let sum = raz.clone().fold_up(Rc::new(|e:&usize|*e),Rc::new(|e1:usize,e2:usize|e1+e2)).unwrap_or(0);
  	let iter_sum: usize = (1..13).sum();
  	assert_eq!(iter_sum, sum);

  	// check the structure
  	let mut cursor = tree::Cursor::from(raz.tree.unwrap());
  	assert!(cursor.down_left());
  	assert!(cursor.down_left());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![1,2], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.down_right());
  	assert!(cursor.down_left());
  	assert!(cursor.down_left());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![3,4], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.down_right());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![5,6], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.down_right());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![7,8], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.down_right());
  	assert!(cursor.down_right());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![11,12], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up() != tree::UpResult::Fail);
  	assert!(cursor.down_left());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![9,10], **v);
  		},
  		_ => panic!("Wrong data")
  	}

  }

	#[test]
	fn test_from_head_stack() {
		// reverse the construction of tail_stack test
		let mut stack = stack::AStack::new();
		stack.push(12);
		stack.push(11);
		stack.archive(Some(name_of_usize(4)),4);
		stack.push(10);
		stack.push(9);
		stack.archive(Some(name_of_usize(5)),5);
		stack.push(8);
		stack.push(7);
		stack.archive(Some(name_of_usize(2)),2);
		stack.push(6);
		stack.push(5);
		stack.archive(Some(name_of_usize(1)),1);
		stack.push(4);
		stack.push(3);
		stack.archive(Some(name_of_usize(3)),3);
		stack.push(2);
		stack.push(1);
		let raz: RazTree<_,Count> = RazTree::memo_from(&stack::AtHead(stack));

		// check that levels are high-to-low
		assert!(good_levels(raz.tree.as_ref().unwrap()));

		// check that all elements are represented
		let sum = raz.clone().fold_up(Rc::new(|e:&usize|*e),Rc::new(|e1:usize,e2:usize|e1+e2)).unwrap_or(0);
		let iter_sum: usize = (1..13).sum();
		assert_eq!(iter_sum, sum);

		// check the structure
		let mut cursor = tree::Cursor::from(raz.tree.unwrap());
		assert!(cursor.down_left());
		assert!(cursor.down_left());
		match cursor.peek() {
			Some(TreeData::Leaf(ref v)) => {
				assert_eq!(vec![1,2], **v);
			},
			_ => panic!("Wrong data")
		}
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.down_right());
		assert!(cursor.down_left());
		assert!(cursor.down_left());
		match cursor.peek() {
			Some(TreeData::Leaf(ref v)) => {
				assert_eq!(vec![3,4], **v);
			},
			_ => panic!("Wrong data")
		}
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.down_right());
		match cursor.peek() {
			Some(TreeData::Leaf(ref v)) => {
				assert_eq!(vec![5,6], **v);
			},
			_ => panic!("Wrong data")
		}
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.down_right());
		match cursor.peek() {
			Some(TreeData::Leaf(ref v)) => {
				assert_eq!(vec![7,8], **v);
			},
			_ => panic!("Wrong data")
		}
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.down_right());
		assert!(cursor.down_right());
		match cursor.peek() {
			Some(TreeData::Leaf(ref v)) => {
				assert_eq!(vec![11,12], **v);
			},
			_ => panic!("Wrong data")
		}
		assert!(cursor.up() != tree::UpResult::Fail);
		assert!(cursor.down_left());
		match cursor.peek() {
			Some(TreeData::Leaf(ref v)) => {
				assert_eq!(vec![9,10], **v);
			},
			_ => panic!("Wrong data")
		}

	}

	#[test]
	fn test_iter() {
		let tree = example_tree();
		let tree_items = tree.into_iter_lr().collect::<Vec<_>>();
		let count_items = (1..13).collect::<Vec<_>>();
		assert_eq!(count_items, tree_items);

		let tree = example_tree();
		let tree_items = tree.into_iter_rl().collect::<Vec<_>>();
		let mut count_items = (1..13).collect::<Vec<_>>();
		count_items.reverse();
		assert_eq!(count_items, tree_items);
	}

  // iters need to be updated
  // #[test]
  // fn test_iters() {
  // 	let mut r = Raz::new();
  // 	let mut t;
  // 	// set same tree as focus example
  // 	r.push_left(3);
  // 	r.push_left(4);
  // 	r.archive_left(1, Some(name_of_usize(1)));
  // 	r.push_right(8);
  // 	r.push_right(7);
  // 	r.archive_right(2, Some(name_of_usize(2)));
  // 	r.push_left(5);
  // 	r.push_right(6);
  // 	t = r.unfocus();
  // 	r = t.focus(0).expect("focus on 0");
  // 	r.push_left(1);
  // 	r.push_left(2);
  // 	r.archive_left(3, Some(name_of_usize(3)));
  // 	t = r.unfocus();

  // 	r = t.focus(8).expect("focus on 8");
  // 	r.archive_left(5, Some(name_of_usize(5)));
  // 	r.push_left(9);
  // 	r.push_left(10);
  // 	r.push_right(12);
  // 	r.push_right(11);
  // 	r.archive_right(4, Some(name_of_usize(4)));
  // 	t = r.unfocus();

  // 	// iterate
  // 	let (l,mut r) = t.focus(8).unwrap().into_iters();
  // 	assert_eq!(Some(9), r.next());
  // 	assert_eq!(Some(10), r.next());
  // 	assert_eq!(2, r.len());
  // 	assert_eq!(Some(11), r.next());
  // 	assert_eq!(Some(12), r.next());
  // 	assert_eq!(None, r.next());
  // 	assert_eq!(None, r.next());
  // 	let mut rev = Vec::new();
  // 	for i in l {
  // 		rev.push(i);
  // 	}
  // 	assert_eq!(vec![8,7,6,5,4,3,2,1], rev);

  // 	// again for the tree iterator
  // 	let mut r = Raz::new();
  // 	let mut t;
  // 	// set same tree as focus example
  // 	r.push_left(3);
  // 	r.push_left(4);
  // 	r.archive_left(1, Some(name_of_usize(1)));
  // 	r.push_right(8);
  // 	r.push_right(7);
  // 	r.archive_right(2, Some(name_of_usize(2)));
  // 	r.push_left(5);
  // 	r.push_right(6);
  // 	t = r.unfocus();
  // 	r = t.focus(0).expect("focus on 0");
  // 	r.push_left(1);
  // 	r.push_left(2);
  // 	r.archive_left(3, Some(name_of_usize(3)));
  // 	t = r.unfocus();

  // 	r = t.focus(8).expect("focus on 8");
  // 	r.archive_left(5, Some(name_of_usize(5)));
  // 	r.push_left(9);
  // 	r.push_left(10);
  // 	r.push_right(12);
  // 	r.push_right(11);
  // 	r.archive_right(4, Some(name_of_usize(4)));
  // 	t = r.unfocus();

		// assert_eq!(
		// 	(1..13).collect::<Vec<_>>(),
		// 	t.into_iter().collect::<Vec<_>>()
		// );  	
  // }

  // fold_lr is old with poor performance
  // #[test]
  // fn test_fold_lr() {
  // 	let mut r = Raz::new();
  // 	let mut t;
  // 	// set same tree as focus example
  // 	r.push_left(3);
  // 	r.push_left(4);
  // 	r.archive_left(1, Some(name_of_usize(1)));
  // 	r.push_right(8);
  // 	r.push_right(7);
  // 	r.archive_right(2, Some(name_of_usize(2)));
  // 	r.push_left(5);
  // 	r.push_right(6);
  // 	t = r.unfocus();
  // 	r = t.focus(0).expect("focus on 0");
  // 	r.push_left(1);
  // 	r.push_left(2);
  // 	r.archive_left(3, Some(name_of_usize(3)));
  // 	t = r.unfocus();
  // 	r = t.focus(8).expect("focus on 8");
  // 	r.archive_left(5, Some(name_of_usize(5)));
  // 	r.push_left(9);
  // 	r.push_left(10);
  // 	r.push_right(12);
  // 	r.push_right(11);
  // 	r.archive_right(4, Some(name_of_usize(4)));
  // 	t = r.unfocus();

  // 	let sum = t.clone().fold_lr(0,Rc::new(|l,r:&usize|{l+*r}));
  // 	let iter_sum: usize = (1..13).sum();
  // 	assert_eq!(iter_sum, sum);

  // 	let raz_string = t.clone().fold_lr("0".to_string(),Rc::new(|l,r:&usize|{format!("{},{}",l,r)}));
  // 	let iter_string = (1..13).collect::<Vec<_>>().iter().fold("0".to_string(),|l,r:&usize|{format!("{},{}",l,r)});
  // 	assert_eq!(iter_string, raz_string);
  // }

  #[test]
  fn test_fold_lr_meta() {
  	let mut r: Raz<_,Count> = Raz::new();
  	let mut t;
  	// set same tree as focus example
  	r.push_left(3);
  	r.push_left(4);
  	r.archive_left(1, Some(name_of_usize(1)));
  	r.push_right(8);
  	r.push_right(7);
  	r.archive_right(2, Some(name_of_usize(2)));
  	r.push_left(5);
  	r.push_right(6);
  	t = r.unfocus();
  	println!("length after 6 inserts: {:?}", t.meta());
  	r = t.focus(0usize).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();
  	println!("length after 2 more: {:?}", t.meta());
  	r = t.focus(8usize).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();
  	println!("length after the last 4: {:?}", t.meta());

  	let sums = ns(name_of_string(String::from("sum")),||{
  		t.clone().fold_lr_meta(
	  		(0,0),
	  		Rc::new(|(lev,dat),e:&u32|{(lev,dat+e)}),
	  		Rc::new(|(lev,dat),(l,_):(u32,Option<Name>)|{(lev+l,dat)})
	  	)});
  	let iter_levs: u32 = (1..6).sum();
  	let iter_sum: u32 = (1..13).sum();
  	assert_eq!((iter_levs,iter_sum), sums);

  	let raz_string = ns(name_of_string(String::from("vals")),||{
  		t.clone().fold_lr_meta(
	  		"s".to_string(),
	  		Rc::new(|l,r:&u32|{format!("{},{}",l,r)}),
	  		Rc::new(|l,_|{format!("{},n",l)}),
  	)});
  	let string = String::from("s,1,2,n,3,4,n,5,6,n,7,8,n,9,10,n,11,12");
  	assert_eq!(string, raz_string);
  }


	#[test]
	fn test_names_many_edits() {
		use rand::{thread_rng,Rng};

		let mut r: Raz<_,Count> = Raz::new();
		let mut t;
		for i in 0..1000 {
			r.push_left(i);
			r.archive_left(tree::gen_level(&mut thread_rng()),Some(name_of_usize(i)));
		}
		t = r.unfocus();

		let original_names = t.clone().fold_lr_meta(
			Vec::new(),
			Rc::new(|a,_e:&usize|{a}),
			Rc::new(|mut a:Vec<Option<Name>>,(_l,n):(_,Option<Name>)|{a.push(n);a}),
		);

		for i in 0..1000 {
			r = t.focus(thread_rng().gen::<usize>() % 1000).unwrap();
			if thread_rng().gen() {
				r.push_left(i);
			} else {
				r.push_right(i);
			}
			t = r.unfocus();
		}

		let new_names = t.clone().fold_lr_meta(
			Vec::new(),
			Rc::new(|a,_e:&usize|{a}),
			Rc::new(|mut a:Vec<Option<Name>>,(_l,n):(_,Option<Name>)|{a.push(n);a}),
		);

		assert_eq!(original_names, new_names);
	}


	#[test]
	fn test_name_indexes() {
		use raz_meta::Names;
		use rand::{thread_rng,Rng};

		let mut r: Raz<_,Names> = Raz::new();
		for i in 0..1000 {
			r.push_left(i);
			if i % 10 == 0 {
				r.archive_left(::inc_level(),Some(name_of_usize(i)));
			}	
		}
		let t = r.unfocus();

		println!("top: {:?}", t.meta());

		for _ in 0..10 {
			let val = (thread_rng().gen::<usize>() % 100) * 10;
			let tree_name = name_of_usize(val);
			println!("{:?}", tree_name);
			let r = t.clone().focus(tree_name).unwrap();
			assert_eq!(Some(val), r.peek_left());
		}
	}

	#[test]
	fn test_peek_pop() {
		let tree = example_tree();

		let mut raz = tree.focus(6usize).unwrap();

		let mut count = 0;
		while let Some(peek) = raz.peek_left() {
			let pop = raz.pop_left().unwrap();
			count += 1;
			assert_eq!(peek, pop);
		}
		assert!(count == 6);
		count = 0;
		while let Some(peek) = raz.peek_right() {
			let pop = raz.pop_right().unwrap();
			count += 1;
			assert_eq!(peek, pop);
		}
		assert!(count == 6);
	}
}
