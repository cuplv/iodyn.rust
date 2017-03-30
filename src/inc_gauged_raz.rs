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
use inc_archive_stack as stack;

use adapton::macros::*;
use adapton::engine::*;

/// Random access zipper
///
/// A cursor into a sequence, optimised for moving to 
/// arbitrary points in the sequence
#[derive(Clone,Eq,PartialEq,Hash,Debug)]
pub struct Raz<E: Debug+Clone+Eq+Hash+'static> {
	l_length: usize,
	r_length: usize,
	l_forest: tree::Cursor<TreeData<E>>,
	l_stack: stack::AStack<E,u32>,
	r_stack: stack::AStack<E,u32>,
	r_forest: tree::Cursor<TreeData<E>>,
}

const DEFAULT_SECTION_CAPACITY: usize = 500;

/// The data stored in the tree structure of the RAZ.
#[derive(PartialEq,Eq,Debug,Hash,Clone)]
enum TreeData<E: Debug+Clone+Eq+Hash> {
	Branch{l_count: usize, r_count: usize},
	Leaf(Rc<Vec<E>>),
}

fn count<E: Debug+Clone+Eq+Hash+'static>(elm: &Option<TreeData<E>>) -> usize {
	match *elm {
		None => 0,
		Some(TreeData::Branch{l_count,r_count}) => l_count + r_count,
		Some(TreeData::Leaf(ref vec)) => vec.len(),
	}
}
fn count_tree_op<E: Debug+Clone+Eq+Hash+'static>(tree: &Option<tree::Tree<TreeData<E>>>) -> usize {
	count(&tree.as_ref().map(|t|t.peek()))
}

impl<E: Debug+Clone+Eq+Hash+'static> tree::TreeUpdate for TreeData<E> {
	#[allow(unused_variables)]
	fn rebuild(l_branch: Option<Self>, old_data: &Self, r_branch: Option<Self>) -> Self {
		match *old_data {
			TreeData::Leaf(ref vec) => TreeData::Leaf(vec.clone()),
			_ => TreeData::Branch{l_count: count(&l_branch), r_count: count(&r_branch)}
		}
	}
}

/// Tree form of a RAZ
///
/// used between refocusing, and for running global algorithms
#[derive(Clone,PartialEq,Eq,Debug)]
pub struct RazTree<E: 'static+Debug+Clone+Eq+Hash>{count: usize, tree: Option<tree::Tree<TreeData<E>>>}

impl<E: Debug+Clone+Eq+Hash+'static> RazTree<E> {
	/// the number if items in the sequence
	pub fn len(&self) -> usize {self.count}

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
					TreeData::Branch{..} => { match (l,r) {
						(None, None) => panic!("branch with no data"),
						(Some(r),None) | (None, Some(r)) => r,
						(Some(r1),Some(r2)) => bin(r1,r2),
					}},
				}
			}))
		})
	}

	/// left-to-right memoized fold (Old version, poor incremental performance)
	pub fn fold_lr<A,B>(self, init: A, bin: Rc<B>) -> A where
		A: 'static + Eq+Clone+Hash+Debug,
		B: 'static + Fn(A,&E) -> A,
	{
		self.into_iter().inc_fold_out(init,bin)
	}

	/// left-to-right memoized fold with levels and names
	pub fn fold_lr_meta<A,B,M>(self, init: A, bin: Rc<B>, meta: Rc<M>) -> A where
		A: 'static + Eq+Clone+Hash+Debug,
		B: 'static + Fn(A,&E) -> A,
		M: 'static + Fn(A,(u32,Option<Name>)) -> A,
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
						TreeData::Branch{..} => {
							meta(a,(l,n))
						},
					}
				}))
			},
		}
	}

	/// returns a new tree with data mapped from the old tree
	pub fn map<R,F>(self, f: Rc<F>) -> RazTree<R> where
		R: 'static + Eq+Clone+Hash+Debug,
		F: 'static + Fn(&E) -> R,
	{
		// TODO: memo! map (only the first rec call is not)
		RazTree{count: self.count, tree:
			self.tree.map(|tree| { // map over Option
				tree.map(Rc::new(move |d|{ // inc map over tree data
					match d {
						TreeData::Leaf(ref vec) => {
							let mapped = vec.iter().map(|e|f(e)).collect();
							TreeData::Leaf(Rc::new(mapped))
						},
						TreeData::Branch{l_count,r_count} => {
							TreeData::Branch{l_count:l_count,r_count:r_count}
						},
					}
				}))
			})
		}
	}


	/// focus on a location in the sequence to begin editing.
	///
	/// `0` is before the first element. This will return `None` if
	/// `pos` is larger than the number of elements.
	pub fn focus(self, mut pos: usize) -> Option<Raz<E>> {
		match self { 
			RazTree{tree:None, ..} => {
				Some(Raz{
					l_length: 0,
					r_length: 0,
					l_forest: tree::Cursor::new(),
					l_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
					r_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
					r_forest: tree::Cursor::new(),
				})
			},
			RazTree{count, tree: Some(tree)} => {
				if count < pos { return None };
				let l_len = pos;
				// step 1: find location with cursor
				let mut cursor = tree::Cursor::from(tree);
				while let TreeData::Branch{l_count, ..} = cursor.peek().unwrap() {
					if pos <= l_count {
						assert!(cursor.down_left());
					} else {
						pos -= l_count;
						assert!(cursor.down_right());
					}
				}
				// step 2: extract and copy data
				let mut l_astack = stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY);
				let mut r_astack = stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY);
				let (l_cursor, tree, r_cursor) = cursor.split();
				match tree {
					None => unreachable!(),
					Some(ref t) => match t.peek() {
						TreeData::Branch{..} => unreachable!(),
						TreeData::Leaf(ref vec_ref) => {
							let (l_slice,r_slice) = vec_ref.split_at(pos);
							l_astack.extend(l_slice);
							r_astack.extend_rev(r_slice);
						}
					}
				};
				// step 3: integrate
				Some(Raz{
					l_length: l_len,
					r_length: count - l_len,
					l_forest: l_cursor,
					l_stack: l_astack,
					r_stack: r_astack,
					r_forest: r_cursor,
				})
			},
		}
	}

}

impl<T: Debug+Clone+Eq+Hash+'static>
IntoIterator for RazTree<T> {
	type Item = T;
	type IntoIter = IterR<T>;
	fn into_iter(self) -> Self::IntoIter {
		IterR(self.focus(0).unwrap())
	}
}

impl<E: Debug+Clone+Eq+Hash+'static>
Raz<E> {
	/// Create a new RAZ, for an empty sequence
	pub fn new() -> Raz<E> {
		Raz{
			l_length: 0,
			r_length: 0,
			l_forest: tree::Cursor::new(),
			l_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
			r_stack: stack::AStack::with_capacity(DEFAULT_SECTION_CAPACITY),
			r_forest: tree::Cursor::new(),
		}
	}

	/// get the total length of the sequence
	pub fn len(&self) -> usize { self.l_length + self.r_length }

	/// unfocus the RAZ before refocusing on a new location
	/// in the sequence.
	pub fn unfocus(mut self) -> RazTree<E> {
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
				let dummy = TreeData::Branch{l_count: 0, r_count: 0};
				cursor = tree::Cursor::join(l_curs,l_lev.unwrap(),l_nm,dummy,cursor);
				l_lev = next_lev;
				l_nm = next_nm;
				next_nm = self.l_stack.name().map(|n|name_pair(n,nmtree.clone()));
			}
			next_nm = self.r_stack.name().map(|n|name_pair(n,nmtree.clone()));
			while let Some((mut r_vec,next_lev)) = self.r_stack.next_archive() {
				r_vec.reverse();
				let r_curs = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(r_vec)),None,None).unwrap().into();
				let dummy = TreeData::Branch{l_count: 0, r_count: 0};
				cursor = tree::Cursor::join(cursor,r_lev.unwrap(),r_nm,dummy,r_curs);
				r_lev = next_lev;
				r_nm = next_nm;
				next_nm = self.r_stack.name().map(|n|name_pair(n,nmtree.clone()));
			}
			while cursor.up() != tree::UpResult::Fail {}
			cursor.at_tree().unwrap()
		} else {
			if self.l_forest.up() == tree::UpResult::Fail {
				if self.r_forest.up() == tree::UpResult::Fail {
					return RazTree{ count: 0, tree: None };
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
			let nm = self.r_forest.peek_name();
			self.l_forest.down_left_force(tree::Force::Discard);
			let dummy = TreeData::Branch{l_count: 0, r_count: 0};
			join_cursor = tree::Cursor::join(self.l_forest,lev,nm,dummy,join_cursor);
		}
		if self.r_forest.up() != tree::UpResult::Fail {
			let lev = self.r_forest.peek_level().unwrap();
			let nm = self.r_forest.peek_name();
			self.r_forest.down_right_force(tree::Force::Discard);
			let dummy = TreeData::Branch{l_count: 0, r_count: 0};
			join_cursor = tree::Cursor::join(join_cursor,lev,nm,dummy,self.r_forest);
		}
		// step 4: convert to final tree
		while join_cursor.up() != tree::UpResult::Fail {}
		let tree = join_cursor.at_tree();
		RazTree{count: count_tree_op(&tree), tree: tree}
	}
  
  /// creates two iterators, one for each side of the cursor
	pub fn into_iters(self) -> (IterL<E>,IterR<E>) {
		match self {
			Raz{
				l_length,
				r_length,
				l_forest,
				l_stack,
				r_stack,
				r_forest,
			} =>
			(IterL(Raz{
				l_length: l_length,
				r_length: 0,
				l_forest: l_forest,
				l_stack: l_stack,
				r_stack: stack::AStack::new(),
				r_forest: tree::Cursor::new(),
			}),
			IterR(Raz{
				l_length: 0,
				r_length: r_length,
				l_forest: tree::Cursor::new(),
				l_stack: stack::AStack::new(),
				r_stack: r_stack,
				r_forest: r_forest,
			}))
		} 
	}

	/// add an element to the left of the cursor
	/// returns number of non-archived elements
	pub fn push_left(&mut self, elm: E) -> usize {
		self.l_length += 1;
		self.l_stack.push(elm);
		self.l_stack.active_len()
	}
	/// add an element to the right of the cursor
	pub fn push_right(&mut self, elm: E) -> usize {
		self.r_length += 1;
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
		if self.l_stack.len() == 0 {
			if self.l_forest.up() == tree::UpResult::Fail { return None } else {
				self.l_forest.down_left_force(tree::Force::Discard);
				while self.l_forest.down_right() {}
				match self.l_forest.peek() {
					Some(TreeData::Leaf(ref data)) => self.l_stack.extend(&***data),
					_ => panic!("pop_left: no left tree leaf"),
				}
			}
		}
		self.l_length -= 1;
		self.l_stack.pop()
	}
	/// remove and return an element to the right of the cursor
	pub fn pop_right(&mut self) -> Option<E> {
		if self.r_stack.len() == 0 {
			if self.r_forest.up() == tree::UpResult::Fail { return None } else {
				self.r_forest.down_right_force(tree::Force::Discard);
				while self.r_forest.down_left() {}
				match self.r_forest.peek() {
					Some(TreeData::Leaf(ref data)) => self.r_stack.extend_rev(&***data),
					_ => panic!("pop_right: no right tree leaf"),
				}
			}
		}
		self.r_length -= 1;
		self.r_stack.pop()
	}
}

pub struct IterL<T: Debug+Clone+Eq+Hash+'static>(Raz<T>);
impl<T: Debug+Clone+Eq+Hash+'static> ExactSizeIterator for IterL<T> {}
impl<T: Debug+Clone+Eq+Hash+'static> Iterator for IterL<T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop_left()
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.0.l_length, Some(self.0.l_length))
	}
}
pub struct IterR<T: Debug+Clone+Eq+Hash+'static>(Raz<T>);
impl<T: Debug+Clone+Eq+Hash+'static> ExactSizeIterator for IterR<T> {}
impl<T: Debug+Clone+Eq+Hash+'static> Iterator for IterR<T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop_right()
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.0.r_length, Some(self.0.r_length))
	}
}
impl<T: Debug+Clone+Eq+Hash+'static> IterR<T> {
	pub fn inc_fold_out<R,B>(self, init:R, bin:Rc<B>) -> R where
		R: 'static + Eq+Clone+Hash+Debug,
		B: 'static + Fn(R,&T) -> R
	{
		match self.0 {Raz{r_stack,mut r_forest, ..}=>{
			let stack_result = r_stack.into_iter().fold(init, |r,t|{bin(r,&t)});
			if r_forest.up_discard() == tree::UpResult::Fail { return stack_result }
			let (_,_,iter) = r_forest.into_iters();
			iter.fold_out(stack_result,Rc::new(move|r,t|{
				match t {
					TreeData::Branch{..} => r,
					TreeData::Leaf(vec) => {
						vec.iter().fold(r,|r,e|{bin(r,e)})
					},
				}
			}))
		}}
	}
}
	

/////////////////////////////
// Traits for Raz and RazTree
/////////////////////////////
use inc_level_tree as ltree;
use std::convert::From;

/// convenience fn for making a tree from data
#[allow(unused)]
fn leaf<E: Debug+Clone+Eq+Hash+'static>(v:Vec<E>, n: Option<Name>) -> ltree::Tree<TreeData<E>> {
	ltree::Tree::new(0,n,TreeData::Leaf(Rc::new(v)),None,None).unwrap()
}
/// convenience fn for combining two trees as branches
#[allow(unused)]
fn bin<E: Debug+Clone+Eq+Hash+'static>(
	t1: ltree::Tree<TreeData<E>>,
	l:  u32,
	n:  Option<Name>,
	t2: ltree::Tree<TreeData<E>>
) -> ltree::Tree<TreeData<E>> {
	ltree::Tree::new(
		l,n, TreeData::Branch{
			l_count: count(&Some(t1.peek())),
			r_count: count(&Some(t2.peek())),
		},
		Some(t1), Some(t2),
	).unwrap()
}

/// Marker type for interpreting the stack as a sequence.
/// 
/// Assume the head of the sequence is the edit point.
/// Rust's default Vec has the edit point at the tail of the data.
#[derive(Clone)]
pub struct AtHead<T: 'static+Debug+Clone+Eq+Hash>(pub stack::AStack<T,u32>);
/// Marker type for interpreting the stack as a sequence.
/// 
/// Assume the tail of the sequence is the edit point.
/// Rust's default Vec has the edit point at the tail of the data.
#[derive(Clone)]
pub struct AtTail<T: 'static+Debug+Clone+Eq+Hash>(pub stack::AStack<T,u32>);

/// Construct `Self` via a memoized conversion
///
/// Rerunning `memo_from` on a slightly modified `T` is expected to take
/// asymptotically less time than the initial run, but with some constant
/// overhead on the initial run
pub trait MemoFrom<T> {
	fn memo_from(T) -> Self;
}

impl<E: Debug+Clone+Eq+Hash+'static>
MemoFrom<AtTail<E>>
for RazTree<E> {
	// we build this tree right to left
	// note that the left branch _cannot_ have
	// the same level as its parent
	// while the right branch can.
	fn memo_from(AtTail(mut tailstack): AtTail<E>) -> Self {
		// memoize from_stack
		fn from_stack_memo<E: Debug+Clone+Eq+Hash+'static>(
			s:stack::AStack<E,u32>, l:u32, n:Option<Name>, t: Tree<TreeData<E>>, m:u32
		) -> (Tree<TreeData<E>>, Option<u32>, Option<Name>, stack::AStack<E,u32>) {
			match n.clone() {
				None => return from_stack(s,l,n,t,m),
				Some(nm) => {
					let nm = name_pair(nm,name_unit());
					return memo!(nm =>> from_stack, s:s, l:l, n:n, t:t, m:m)
				},
			}
		}
		// main function, uses memoized recursive calls from previous function 
		fn from_stack<E: Debug+Clone+Eq+Hash+'static>(
			mut stack: stack::AStack<E,u32>,
			first_level: u32,
			first_name: Option<Name>,
			accum_tree: Tree<TreeData<E>>,
			max_level: u32,
		) -> (Tree<TreeData<E>>, Option<u32>, Option<Name>, stack::AStack<E,u32>) {
			assert!(accum_tree.level() <= first_level);
			if first_level < max_level {
				let next_name = stack.name();
				let (vec,next_level) = stack.next_archive().unwrap_or_else(||{panic!("stack was unexpectedly empty")});
				let leaf_tree = leaf(vec,None);
				let (small_tree, final_level,final_name,shorter_stack) = match next_level {
					None => (leaf_tree,None,None,stack::AStack::new()),
					Some(lev) => from_stack_memo(stack,lev,next_name,leaf_tree,first_level),
				};
				let new_tree = bin(small_tree, first_level, first_name, accum_tree);
				match final_level {
					None => return (new_tree, None, None, stack::AStack::new()),
					Some(lev) => from_stack_memo(shorter_stack,lev,final_name,new_tree,max_level),
				}
			} else {
				// hack to avoid name collisions
				// TODO: make sure every bin() gets an input name, and memo's get forked names
				let nm = first_name.map(|n|{name_fork(n).1});
				return (accum_tree, Some(first_level), nm, stack);
			}
		}
		let name = tailstack.name();
		let (level, first_tree) = match tailstack.next_archive() {
			None => return RazTree{count:0, tree: None},
			Some((vec,None)) => return RazTree{count: vec.len(), tree: Some(leaf(vec,None))},
			Some((vec,Some(level))) => (level,leaf(vec,None))
		};
		let (t,l,n,s) = from_stack_memo(tailstack, level, name, first_tree, u32::max_value());
		assert!(l.is_none());
		assert!(n.is_none());
		assert!(s.is_empty());
		RazTree{count: count(&Some(t.peek())), tree: Some(t)}
	}
}



////////////////////////////////
// Zip and ZipSeq for Gauged RAZ 
////////////////////////////////

use zip::Zip;
use seqzip::{Seq, SeqZip};

impl<E: Debug+Clone+Eq+Hash+'static> Zip<E> for Raz<E> {
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

impl<E: Debug+Clone+Eq+Hash+'static> Seq<E, Raz<E>> for RazTree<E> {
	fn zip_to(&self, loc: usize) -> Result<Raz<E>,&str> {
		self.clone().focus(loc).ok_or("Gauged RAZ: focus out of range")
	}
}

impl<E: Debug+Clone+Eq+Hash+'static> SeqZip<E, RazTree<E>> for Raz<E> {
	fn unzip(&self) -> RazTree<E> {
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

  #[test]
  fn test_push_pop() {
  	let mut raz = Raz::new();
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
  	let tree = RazTree{
  		count: 12,
  		tree: tree::Tree::new(5, Some(name_of_usize(5)),TreeData::Branch{l_count:8, r_count: 4},
  			tree::Tree::new(3, Some(name_of_usize(3)),TreeData::Branch{l_count:2, r_count: 6},
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(1,2))),None,None),
  				tree::Tree::new(2, Some(name_of_usize(2)),TreeData::Branch{l_count:4, r_count: 2},
  					tree::Tree::new(1, Some(name_of_usize(1)),TreeData::Branch{l_count:2, r_count: 2},
		  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(3,4))),None,None),
		  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(5,6))),None,None),
  					),
  					tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(7,8))),None,None),
  				)
  			),
  			tree::Tree::new(4, Some(name_of_usize(4)),TreeData::Branch{l_count: 2, r_count: 2},
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(9,10))),None,None),
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(11,12))),None,None),
  			)
  		)
  	};
  	assert!(good_levels(tree.tree.as_ref().unwrap()));

  	let mut left = tree.clone().focus(0).unwrap();
  	let mut deep = tree.clone().focus(5).unwrap();
  	let mut right = tree.clone().focus(12).unwrap();

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
  	let mut r = Raz::new();
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
  	r = t.focus(0).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();

  	r = t.focus(8).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();

  	assert!(good_levels(t.tree.as_ref().unwrap()));

  	// focus and read
  	r = t.focus(7).expect("focus on 7");
  	assert_eq!(Some(7), r.pop_left());
  	assert_eq!(Some(6), r.pop_left());
  	assert_eq!(Some(5), r.pop_left());
  	assert_eq!(Some(4), r.pop_left());
  	assert_eq!(Some(3), r.pop_left());
  	assert_eq!(Some(2), r.pop_left());
  	assert_eq!(Some(1), r.pop_left());
  	t = r.unfocus();
  	r = t.focus(5).expect("focus on 5");
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
  	let tree = RazTree{
  		count: 12,
  		tree: tree::Tree::new(5, Some(name_of_usize(5)),TreeData::Branch{l_count:8, r_count: 4},
  			tree::Tree::new(3, Some(name_of_usize(3)),TreeData::Branch{l_count:2, r_count: 6},
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(1,2))),None,None),
  				tree::Tree::new(2, Some(name_of_usize(2)),TreeData::Branch{l_count:4, r_count: 2},
  					tree::Tree::new(1, Some(name_of_usize(1)),TreeData::Branch{l_count:2, r_count: 2},
		  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(3,4))),None,None),
		  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(5,6))),None,None),
  					),
  					tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(7,8))),None,None),
  				)
  			),
  			tree::Tree::new(4, Some(name_of_usize(4)),TreeData::Branch{l_count: 2, r_count: 2},
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(9,10))),None,None),
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(11,12))),None,None),
  			)
  		)
  	};
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
  	let tree = RazTree{
  		count: 12,
  		tree: tree::Tree::new(5, Some(name_of_usize(5)),TreeData::Branch{l_count:8, r_count: 4},
  			tree::Tree::new(3, Some(name_of_usize(3)),TreeData::Branch{l_count:2, r_count: 6},
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(1,2))),None,None),
  				tree::Tree::new(2, Some(name_of_usize(2)),TreeData::Branch{l_count:4, r_count: 2},
  					tree::Tree::new(1, Some(name_of_usize(1)),TreeData::Branch{l_count:2, r_count: 2},
		  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(3,4))),None,None),
		  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(5,6))),None,None),
  					),
  					tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(7,8))),None,None),
  				)
  			),
  			tree::Tree::new(4, Some(name_of_usize(4)),TreeData::Branch{l_count: 2, r_count: 2},
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(9,10))),None,None),
  				tree::Tree::new(0, None,TreeData::Leaf(Rc::new(vec!(11,12))),None,None),
  			)
  		)
  	};

  	let plus1 = tree.map(Rc::new(|e: &usize|*e+1));
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
  fn test_from_stack() {
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
  	let raz = RazTree::memo_from(AtTail(stack));

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
  fn test_iters() {
  	let mut r = Raz::new();
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
  	r = t.focus(0).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();

  	r = t.focus(8).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();

  	// iterate
  	let (l,mut r) = t.focus(8).unwrap().into_iters();
  	assert_eq!(Some(9), r.next());
  	assert_eq!(Some(10), r.next());
  	assert_eq!(2, r.len());
  	assert_eq!(Some(11), r.next());
  	assert_eq!(Some(12), r.next());
  	assert_eq!(None, r.next());
  	assert_eq!(None, r.next());
  	let mut rev = Vec::new();
  	for i in l {
  		rev.push(i);
  	}
  	assert_eq!(vec![8,7,6,5,4,3,2,1], rev);

  	// again for the tree iterator
  	let mut r = Raz::new();
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
  	r = t.focus(0).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();

  	r = t.focus(8).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();

		assert_eq!(
			(1..13).collect::<Vec<_>>(),
			t.into_iter().collect::<Vec<_>>()
		);  	
  }

  #[test]
  fn test_fold_lr() {
  	let mut r = Raz::new();
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
  	r = t.focus(0).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();
  	r = t.focus(8).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();

  	let sum = t.clone().fold_lr(0,Rc::new(|l,r:&usize|{l+*r}));
  	let iter_sum: usize = (1..13).sum();
  	assert_eq!(iter_sum, sum);

  	let raz_string = t.clone().fold_lr("0".to_string(),Rc::new(|l,r:&usize|{format!("{},{}",l,r)}));
  	let iter_string = (1..13).collect::<Vec<_>>().iter().fold("0".to_string(),|l,r:&usize|{format!("{},{}",l,r)});
  	assert_eq!(iter_string, raz_string);
  }

  #[test]
  fn test_fold_lr_meta() {
  	let mut r = Raz::new();
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
  	r = t.focus(0).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3, Some(name_of_usize(3)));
  	t = r.unfocus();
  	r = t.focus(8).expect("focus on 8");
  	r.archive_left(5, Some(name_of_usize(5)));
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4, Some(name_of_usize(4)));
  	t = r.unfocus();

  	let sums = t.clone().fold_lr_meta(
  		name_of_string(String::from("sum")),
  		(0,0),
  		Rc::new(|(lev,dat),e:&u32|{(lev,dat+e)}),
  		Rc::new(|(lev,dat),(l,_):(u32,Option<Name>)|{(lev+l,dat)})
  	);
  	let iter_levs: u32 = (1..6).sum();
  	let iter_sum: u32 = (1..13).sum();
  	assert_eq!((iter_levs,iter_sum), sums);

  	let raz_string = t.clone().fold_lr_meta(
  		name_of_string(String::from("vals")),
  		"s".to_string(),
  		Rc::new(|l,r:&u32|{format!("{},{}",l,r)}),
  		Rc::new(|l,_|{format!("{},n",l)}),
  	);
  	let string = String::from("s,1,2,n,3,4,n,5,6,n,7,8,n,9,10,n,11,12");
  	assert_eq!(string, raz_string);
  }

}
