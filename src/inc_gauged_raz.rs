//! Temporary alteration of guaged_raz for incremental use
//!
//! Gauged RAZ - random access sequence
//! - cursor access in low-const O(1) time
//! - arbirary access in O(log n) time
//! - combines tree cursor with stack

use std::rc::Rc;

use std::fmt::Debug;
use std::hash::Hash;

use inc_level_tree::{Tree};
use inc_tree_cursor as tree;
use inc_tree_cursor::TreeUpdate;
use inc_archive_stack as stack;
use raz_meta::{RazMeta,Navigation,Count,FirstLast};
use memo::{MemoFrom};

use adapton::macros::*;
use adapton::engine::*;

/// Random access zipper
///
/// A cursor into a sequence, optimised for moving to 
/// arbitrary points in the sequence
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

impl<E: Debug+Clone+Eq+Hash+'static, M:RazMeta<E>> RazTree<E,M> {
	/// get the meta data
	pub fn meta(&self) -> &M {&self.meta}
	pub fn is_empty(&self) -> bool {self.tree.is_none()}

	pub fn empty() -> Self {
		RazTree{meta: treetop_meta(None), tree: None}
	}

	/// Combine two trees left to right
	///
	/// returns None if either tree is empty or the levels
	/// are inappropriate. The level of the left side should
	/// be lower than the given level, and the level of the
	/// right side should be equal or lower than the given level.
	// TODO: Deal with bad levels.
	pub fn join(ltree: Self, level: u32, name: Option<Name>, rtree: Self) -> Option<Self> {
		let tree = match (ltree,rtree) {
			(RazTree{tree:Some(lt),..},RazTree{tree:Some(rt),..}) => {
				// if lt.level() < level && rt.level() <= level {
					// there's a level check through bin in Tree::new()
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

	/// Runs an binary function over the sequence data
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

	/// Runs an binary function over the sequence data, levels, and names
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

	/// left-to-right memoized fold with levels and names, with a name provided at the leaf
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

	/// returns a new tree with data mapped from the old tree
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

}

// impl<T: Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
// IntoIterator for RazTree<T,M> {
// 	type Item = T;
// 	type IntoIter = IterR<T>;
// 	fn into_iter(self) -> Self::IntoIter {
// 		IterR(self.focus_left())
// 	}
// }

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

	/// unfocus the RAZ before refocusing on a new location
	/// in the sequence.
	pub fn unfocus(mut self) -> RazTree<E,M> {
		let nmtree = name_of_string(String::from("tree"));
		let mut l_lev = None;
		let mut r_lev = None;
		let mut l_nm;
		let mut r_nm;
		// step 1: reconstruct local array from stack
		l_nm = self.l_stack.name().map(|n|name_pair(n,nmtree.clone()));
		let l_vec = if let Some((vec,lev)) = self.l_stack.next_archive() {
			l_lev = lev;
			if vec.len() > 0 {Some(vec)} else {None}
		} else { None };
		r_nm = self.r_stack.name().map(|n|name_pair(n,nmtree.clone()));
		let r_vec = if let Some((vec,lev)) = self.r_stack.next_archive() {
			r_lev = lev;
			if vec.len() > 0 {Some(vec)} else {None}
		} else { None };
		let vec = match (self.l_stack.is_empty(), l_vec, r_vec, self.r_stack.is_empty()) {
			(_,Some(v),None,_) => Some(v),
			(_,None,Some(mut v),_) => { v.reverse(); Some(v) },
			(_,Some(mut lv),Some(mut rv),_) => {rv.reverse(); lv.extend(rv); Some(lv) },
			(false, None, None, _) => {
				l_nm = self.l_stack.name().map(|n|name_pair(n,nmtree.clone()));
				let (v,lev) = self.l_stack.next_archive().unwrap();
				l_lev = lev;
				Some(v)
			},
			(true, None, None, false) => {
				r_nm = self.r_stack.name().map(|n|name_pair(n,nmtree.clone()));
				let (mut v,lev) = self.r_stack.next_archive().unwrap();
				v.reverse();
				r_lev = lev;
				Some(v)
			},
			_ => None
		};
		// step 2: build center tree
		let tree = if let Some(v) = vec {
			let mut cursor = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(v)),None,None).unwrap().into();
			let mut next_nm = self.l_stack.name().map(|n|name_pair(n,nmtree.clone()));
			while let Some((l_vec,next_lev)) = self.l_stack.next_archive() {
				let l_curs = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(l_vec)),None,None).unwrap().into();
				cursor = tree::Cursor::join(l_curs,l_lev.unwrap(),l_nm,TreeData::Dummy,cursor);
				l_lev = next_lev;
				l_nm = next_nm;
				next_nm = self.l_stack.name().map(|n|name_pair(n,nmtree.clone()));
			}
			next_nm = self.r_stack.name().map(|n|name_pair(n,nmtree.clone()));
			while let Some((mut r_vec,next_lev)) = self.r_stack.next_archive() {
				r_vec.reverse();
				let r_curs = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(r_vec)),None,None).unwrap().into();
				cursor = tree::Cursor::join(cursor,r_lev.unwrap(),r_nm,TreeData::Dummy,r_curs);
				r_lev = next_lev;
				r_nm = next_nm;
				next_nm = self.r_stack.name().map(|n|name_pair(n,nmtree.clone()));
			}
			while cursor.up() != tree::UpResult::Fail {}
			cursor.at_tree().unwrap()
		} else {
			if self.l_forest.up() == tree::UpResult::Fail {
				if self.r_forest.up() == tree::UpResult::Fail {
					return RazTree{ meta: treetop_meta(None), tree: None };
				} else {
					self.r_forest.right_tree().unwrap()
				}
			} else {
				self.l_forest.left_tree().unwrap()
			}
		};
		// step 3: join with forests
		let mut join_cursor = tree::Cursor::from(tree);
		if self.l_forest.up() != tree::UpResult::Fail {
			let lev = self.l_forest.peek_level().unwrap();
			let nm = self.l_forest.peek_name();
			self.l_forest.down_left_force(tree::Force::Discard);
			join_cursor = tree::Cursor::join(self.l_forest,lev,nm,TreeData::Dummy,join_cursor);
		}
		if self.r_forest.up() != tree::UpResult::Fail {
			let lev = self.r_forest.peek_level().unwrap();
			let nm = self.r_forest.peek_name();
			self.r_forest.down_right_force(tree::Force::Discard);
			join_cursor = tree::Cursor::join(join_cursor,lev,nm,TreeData::Dummy,self.r_forest);
		}
		// step 4: convert to final tree
		while join_cursor.up() != tree::UpResult::Fail {}
		let tree = join_cursor.at_tree();
		RazTree{meta: treetop_meta(tree.as_ref()), tree: tree}
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
	/// returns number of non-archived elements
	pub fn push_left(&mut self, elm: E) -> usize {
		self.l_stack.push(elm);
		self.l_stack.active_len()
	}
	/// add an element to the right of the cursor
	pub fn push_right(&mut self, elm: E) -> usize {
		self.r_stack.push(elm);
		self.r_stack.active_len()
	}
	/// peek at the element to the left of the cursor
	pub fn peek_left(&self) -> Option<E> {
		self.l_stack.peek()
	}
	/// peek at the element to the left of the cursor
	pub fn peek_right(&self) -> Option<E> {
		self.r_stack.peek()
	}
	/// mark the data at the left to be shared
	pub fn archive_left(&mut self, level: u32, name: Option<Name>) {
		self.l_stack.archive(name,level);
	}
	/// mark the data at the right to be shared
	pub fn archive_right(&mut self, level: u32, name: Option<Name>) {
		self.r_stack.archive(name,level);
	}

	/// remove and return an element to the left of the cursor
	pub fn pop_left(&mut self) -> Option<E> {
		if self.l_stack.is_empty() {
			if self.l_forest.up() == tree::UpResult::Fail { return None } else {
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
	pub fn pop_right(&mut self) -> Option<E> {
		if self.r_stack.is_empty() {
			if self.r_forest.up() == tree::UpResult::Fail { return None } else {
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
}

// pub struct IterL<T: Debug+Clone+Eq+Hash+'static>(Raz<T>);
// impl<T: Debug+Clone+Eq+Hash+'static> Iterator for IterL<T> {
// 	type Item = T;
// 	fn next(&mut self) -> Option<Self::Item> {
// 		unimplemented!() // don't change the data!
// 	}
// }
// pub struct IterR<T: Debug+Clone+Eq+Hash+'static>(Raz<T>);
// impl<T: Debug+Clone+Eq+Hash+'static> Iterator for IterR<T> {
// 	type Item = T;
// 	fn next(&mut self) -> Option<Self::Item> {
// 		unimplemented!() // don't change the data!
// 	}
// }
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
use inc_level_tree as ltree;
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

impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
MemoFrom<stack::AtTail<E,u32>>
for RazTree<E,M> {
	// we build this tree right to left
	// note that the left branch _cannot_ have
	// the same level as its parent
	// while the right branch can.
	fn memo_from(tailstack: &stack::AtTail<E,u32>) -> Self {
		// memoize from_stack
		fn from_stack_memo<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
			s:stack::AStack<E,u32>, l:u32, n:Option<Name>, t: Tree<TreeData<E,M>>, m:u32
		) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
			match n.clone() {
				None => return from_stack(s,l,n,t,m),
				Some(nm) => {
					let (nm,_) = name_fork(nm);
					return memo!(nm =>> from_stack, s:s, l:l, n:n, t:t, m:m)
				},
			}
		}
		// main function, uses memoized recursive calls from previous function 
		fn from_stack<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
			mut stack: stack::AStack<E,u32>,
			first_level: u32,
			first_name: Option<Name>,
			accum_tree: Tree<TreeData<E,M>>,
			max_level: u32,
		) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
			assert!(accum_tree.level() <= first_level);
			assert!(first_level < max_level);
			let next_name = stack.name();
			let (vec,next_level) = stack.next_archive().unwrap_or_else(||{panic!("stack was unexpectedly empty")});
			let leaf_tree = leaf(vec,None);
			let (shorter_stack, final_level,final_name,small_tree) = match next_level {
				None =>
					(stack::AStack::new(),None,None,leaf_tree),
				Some(lev) => if lev < first_level {
					from_stack_memo(stack,lev,next_name,leaf_tree,first_level)
				} else {
					(stack,Some(lev),next_name,leaf_tree)
				},
			};
			let new_tree = bin(small_tree, first_level, first_name, accum_tree);
			match final_level {
				None =>
					(stack::AStack::new(), None, None, new_tree),
				Some(lev) => if lev < max_level {
					from_stack_memo(shorter_stack,lev,final_name,new_tree,max_level)
				} else {
					(shorter_stack,Some(lev),final_name,new_tree)
				},
			}
		}
		let mut tailstack = tailstack.0.clone();
		let name = tailstack.name();
		let (level, first_tree) = match tailstack.next_archive() {
			None => return RazTree{meta: treetop_meta(None), tree: None},
			Some((vec,None)) => {
				let t = Some(leaf(vec,None));
				return RazTree{meta: treetop_meta(t.as_ref()), tree: t}
			},
			Some((vec,Some(level))) => (level,leaf(vec,None))
		};
		let (s,l,n,t) = from_stack_memo(tailstack, level, name, first_tree, u32::max_value());
		assert!(l.is_none());
		assert!(n.is_none());
		assert!(s.is_empty());
		RazTree{meta: treetop_meta(Some(&t)), tree: Some(t)}
	}
}

impl<E:Debug+Clone+Eq+Hash+'static, M:RazMeta<E>>
MemoFrom<stack::AtHead<E,u32>>
for RazTree<E,M> {
	// we build this tree left to right
	// note that the left branch _cannot_ have
	// the same level as its parent
	// while the right branch can.
	//
	// vecs from the data are reversed, because Vec pushes
	// to the tail, but this stack was used as if pushing
	// to head. The head of a Raz in on the left.
	fn memo_from(headstack: &stack::AtHead<E,u32>) -> Self {
		// memoize from_stack
		fn from_stack_memo<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
			s:stack::AStack<E,u32>, l:u32, n:Option<Name>, t: Tree<TreeData<E,M>>, m:u32
		) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
			match n.clone() {
				None => return from_stack(s,l,n,t,m),
				Some(nm) => {
					let (nm,_) = name_fork(nm);
					return memo!(nm =>> from_stack, s:s, l:l, n:n, t:t, m:m)
				},
			}
		}
		// main function, uses memoized recursive calls from previous function
		fn from_stack<E: Debug+Clone+Eq+Hash+'static,M:RazMeta<E>>(
			mut stack: stack::AStack<E,u32>,
			first_level: u32,
			first_name: Option<Name>,
			accum_tree: Tree<TreeData<E,M>>,
			max_level: u32,
		) -> (stack::AStack<E,u32>, Option<u32>, Option<Name>, Tree<TreeData<E,M>>) {
			assert!(accum_tree.level() < first_level);
			assert!(first_level <= max_level);
			let next_name = stack.name();
			let (mut vec,next_level) = stack.next_archive().unwrap_or_else(||{panic!("stack was unexpectedly empty")});
			vec.reverse();
			let leaf_tree = leaf(vec,None);
			let (shorter_stack, final_level,final_name,small_tree) = match next_level {
				None =>
					(stack::AStack::new(),None,None,leaf_tree),
				Some(lev) => if lev <= first_level {
					from_stack_memo(stack,lev,next_name,leaf_tree,first_level)
				} else {
					(stack,Some(lev),next_name,leaf_tree)
				},
			};
			let new_tree = bin(accum_tree, first_level, first_name, small_tree);
			match final_level {
				None =>
					(stack::AStack::new(), None, None, new_tree),
				Some(lev) => if lev <= max_level {
					from_stack_memo(shorter_stack,lev,final_name,new_tree,max_level)
				} else {
					(shorter_stack,Some(lev),final_name,new_tree)
				},
			}
		}
		let mut headstack = headstack.0.clone();
		let name = headstack.name();
		let (level, first_tree) = match headstack.next_archive() {
			None => return RazTree{meta: treetop_meta(None), tree: None},
			Some((mut vec,None)) => {
				vec.reverse();
				let t = Some(leaf(vec,None));
				return RazTree{meta: treetop_meta(t.as_ref()), tree: t}
			},
			Some((mut vec,Some(level))) => {
				vec.reverse();
				(level,leaf(vec,None))
			}
		};
		let (s,l,n,t) = from_stack_memo(headstack, level, name, first_tree, u32::max_value());
		assert!(l.is_none());
		assert!(n.is_none());
		assert!(s.is_empty());
		RazTree{meta: treetop_meta(Some(&t)), tree: Some(t)}
	}
}



////////////////////////////////
// Zip and ZipSeq for Gauged RAZ 
////////////////////////////////

use zip::Zip;
use seqzip::{Seq, SeqZip};

impl<E:Debug+Clone+Eq+Hash+'static> Zip<E> for Raz<E,Count> {
	fn peek_l(&self) -> Result<E,&str> {
		self.peek_left().map(|elm| elm.clone()).ok_or("Gauged RAZ: no elements to peek at")
	}
	fn peek_r(&self) -> Result<E,&str> {
		self.peek_right().map(|elm| elm.clone()).ok_or("Gauged RAZ: no elements to peek at")
	}
	fn push_l(&self, val: E) -> Self {
		let mut raz = self.clone();
		raz.push_left(val);
		raz
	}
	fn push_r(&self, val: E) -> Self {
		let mut raz = self.clone();
		raz.push_right(val);
		raz
	}
	fn pull_l(&self) -> Result<Self,&str> {
		let mut raz = self.clone();
		match raz.pop_left() {
			Some(_) => Ok(raz),
			None => Err("Gauged RAZ: no elements to remove")
		}
	}
	fn pull_r(&self) -> Result<Self,&str> {
		let mut raz = self.clone();
		match raz.pop_right() {
			Some(_) => Ok(raz),
			None => Err("Gauged RAZ: no elements to remove")
		}
	}
}

impl<E: Debug+Clone+Eq+Hash+'static> Seq<E, Raz<E,Count>> for RazTree<E,Count> {
	fn zip_to(&self, loc: usize) -> Result<Raz<E,Count>,&str> {
		self.clone().focus(loc).ok_or("Gauged RAZ: focus out of range")
	}
}

impl<E: Debug+Clone+Eq+Hash+'static> SeqZip<E, RazTree<E,Count>> for Raz<E,Count> {
	fn unzip(&self) -> RazTree<E,Count> {
		self.clone().unfocus()
	}
}



///////////////////////
// Tests for Gauged RAZ
///////////////////////


#[cfg(test)]
mod tests {
	use super::*;
	use inc_level_tree::good_levels;

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

		//println!("top: {:?}", t.meta());

		for _ in 0..10 {
			let val = (thread_rng().gen::<usize>() % 100) * 10;
			let tree_name = name_pair(name_of_usize(val),name_of_string(String::from("tree")));
			//println!("{:?}", tree_name);
			let r = t.clone().focus(tree_name).unwrap();
			assert_eq!(Some(val), r.peek_left());
		}
	}
}
