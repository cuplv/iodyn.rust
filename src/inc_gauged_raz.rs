//! Temporary alteration of guaged_raz for incremental use
//!
//! Gauged RAZ - random access sequence
//! - cursor access in low-const O(1) time
//! - arbirary access in O(log n) time
//! - combines tree cursor with stack

use std::rc::Rc;

use std::fmt::Debug;
use std::hash::Hash;

use inc_tree_cursor as tree;
use archive_stack as stack;

use adapton::engine::Name;

/// Random access zipper
///
/// A cursor into a sequence, optimised for moving to 
/// arbitrary points in the sequence
#[derive(Clone)]
pub struct Raz<E: Debug+Clone+Eq+Hash+'static> {
	l_length: usize,
	r_length: usize,
	l_forest: tree::Cursor<TreeData<E>>,
	l_stack: stack::AStack<E,(u32,Option<Name>)>,
	r_stack: stack::AStack<E,(u32,Option<Name>)>,
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
		TreeData::Branch{l_count: count(&l_branch), r_count: count(&r_branch)}
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
	pub fn fold_up<I,R,B>(self, init: Rc<I>, bin: Rc<B>) -> Option<R>
		where
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

	/// returns a new tree with data mapped from the old tree
	pub fn map<R,F>(self, f: Rc<F>) -> RazTree<R>
		where
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

impl<E: Debug+Clone+Eq+Hash+'static> Raz<E> {
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
		let mut l_lev = None;
		let mut r_lev = None;
		let mut l_nm = None;
		let mut r_nm = None;
		// step 1: reconstruct local array from stack
		let l_vec = if let Some((vec,lev_nm)) = self.l_stack.next_archive() {
			let (lev,nm) = match lev_nm { None => (None,None), Some((lev,nm)) => (Some(lev),nm)};
			l_lev = lev;
			l_nm = nm;
			if vec.len() > 0 {Some(vec)} else {None}
		} else { None };
		let r_vec = if let Some((vec,lev_nm)) = self.r_stack.next_archive() {
			let (lev,nm) = match lev_nm { None => (None,None), Some((lev,nm)) => (Some(lev),nm)};
			r_lev = lev;
			r_nm = nm;
			if vec.len() > 0 {Some(vec)} else {None}
		} else { None };
		let vec = match (self.l_stack.is_empty(), l_vec, r_vec, self.r_stack.is_empty()) {
			(_,Some(v),None,_) => Some(v),
			(_,None,Some(mut v),_) => { v.reverse(); Some(v) },
			(_,Some(mut lv),Some(mut rv),_) => {rv.reverse(); lv.extend(rv); Some(lv) },
			(false, None, None, _) => {
				let (v,lev_nm) = self.l_stack.next_archive().unwrap();
				let (lev,nm) = match lev_nm { None => (None,None), Some((lev,nm)) => (Some(lev),nm)};
				l_lev = lev;
				l_nm = nm;
				Some(v)
			},
			(true, None, None, false) => {
				let (mut v,lev_nm) = self.r_stack.next_archive().unwrap();
				v.reverse();
				let (lev,nm) = match lev_nm { None => (None,None), Some((lev,nm)) => (Some(lev),nm)};
				r_lev = lev;
				r_nm = nm;
				Some(v)
			},
			_ => None
		};
		// step 2: build center tree
		let tree = if let Some(v) = vec {
			// OPTIMIZE: linear algorithm
			let mut cursor = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(v)),None,None).unwrap().into();
			while let Some((l_vec,lev_nm)) = self.l_stack.next_archive() {
				let (lev,nm) = match lev_nm { None => (None,None), Some((lev,nm)) => (Some(lev),nm)};
				let l_curs = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(l_vec)),None,None).unwrap().into();
				let dummy = TreeData::Branch{l_count: 0, r_count: 0};
				cursor = tree::Cursor::join(l_curs,l_lev.unwrap(),l_nm,dummy,cursor);
				l_lev = lev;
				l_nm = nm;
			}
			while let Some((mut r_vec,lev_nm)) = self.r_stack.next_archive() {
				let (lev,nm) = match lev_nm { None => (None,None), Some((lev,nm)) => (Some(lev),nm)};
				r_vec.reverse();
				let r_curs = tree::Tree::new(0,None,TreeData::Leaf(Rc::new(r_vec)),None,None).unwrap().into();
				let dummy = TreeData::Branch{l_count: 0, r_count: 0};
				cursor = tree::Cursor::join(cursor,r_lev.unwrap(),r_nm,dummy,r_curs);
				r_lev = lev;
				r_nm = nm;
			}
			while cursor.up() {}
			cursor.at_tree().unwrap()
		} else {
			if !self.l_forest.up() {
				if !self.r_forest.up() {
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
		if self.l_forest.up() {
			let lev = self.l_forest.peek_level().unwrap();
			let nm = self.r_forest.peek_name();
			self.l_forest.down_left_force(tree::Force::Discard);
			let dummy = TreeData::Branch{l_count: 0, r_count: 0};
			join_cursor = tree::Cursor::join(self.l_forest,lev,nm,dummy,join_cursor);
		}
		if self.r_forest.up() {
			let lev = self.r_forest.peek_level().unwrap();
			let nm = self.r_forest.peek_name();
			self.r_forest.down_right_force(tree::Force::Discard);
			let dummy = TreeData::Branch{l_count: 0, r_count: 0};
			join_cursor = tree::Cursor::join(join_cursor,lev,nm,dummy,self.r_forest);
		}
		// step 4: convert to final tree
		while join_cursor.up() {}
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
	pub fn peek_left(&self) -> Option<&E> {
		self.l_stack.peek()
	}
	/// peek at the element to the left of the cursor
	pub fn peek_right(&self) -> Option<&E> {
		self.r_stack.peek()
	}
	/// mark the data at the left to be shared
	pub fn archive_left(&mut self, level: u32, name: Option<Name>) {
		self.l_stack.archive((level,name));
	}
	/// mark the data at the right to be shared
	pub fn archive_right(&mut self, level: u32, name: Option<Name>) {
		self.r_stack.archive((level,name));
	}
	/// remove and return an element to the left of the cursor
	pub fn pop_left(&mut self) -> Option<E> {
		if self.l_stack.len() == 0 {
			if !self.l_forest.up() { return None } else {
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
			if !self.r_forest.up() { return None } else {
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
impl<T: Debug+Clone+Eq+Hash+'static> Iterator for IterL<T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop_left()
	}
}
pub struct IterR<T: Debug+Clone+Eq+Hash+'static>(Raz<T>);
impl<T: Debug+Clone+Eq+Hash+'static> Iterator for IterR<T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop_right()
	}
}

/////////////////////////////
// Traits for Raz and RazTree
/////////////////////////////
use inc_level_tree as ltree;
use std::convert::From;
use std::ops::Deref;

/// convenience fn for making a tree from data
fn leaf<E: Debug+Clone+Eq+Hash+'static>(v:Vec<E>, n: Option<Name>) -> ltree::Tree<TreeData<E>> {
	ltree::Tree::new(0,n,TreeData::Leaf(Rc::new(v)),None,None).unwrap()
}
/// convenience fn for combining two trees as branches
fn bin<E: Debug+Clone+Eq+Hash+'static>(
	t1: ltree::Tree<TreeData<E>>,
	l:  u32,
	n:  Option<Name>,
	t2: ltree::Tree<TreeData<E>>
) -> ltree::Tree<TreeData<E>> {
	ltree::Tree::new(
		l,n, TreeData::Branch{l_count:count(&Some(t1.peek())), r_count: count(&Some(t2.peek()))},
		Some(t1), Some(t2)
	).unwrap()
}

/// Marker type for interpreting the stack as a sequence.
/// 
/// Assume the head of the sequence is the edit point.
/// Rust's default Vec has the edit point at the tail of the data.
#[derive(Clone)]
pub struct AtHead<T: Debug+Clone+Eq+Hash>(pub stack::AStack<T,(u32,Option<Name>)>);
/// Marker type for interpreting the stack as a sequence.
/// 
/// Assume the tail of the sequence is the edit point.
/// Rust's default Vec has the edit point at the tail of the data.
#[derive(Clone)]
pub struct AtTail<T: Debug+Clone+Eq+Hash>(pub stack::AStack<T,(u32,Option<Name>)>);
impl<T: Debug+Clone+Eq+Hash> Deref for AtHead<T> {
	type Target = stack::AStack<T,(u32,Option<Name>)>;
	fn deref(&self) -> &Self::Target { &self.0 }
}
impl<T: Debug+Clone+Eq+Hash> Deref for AtTail<T> {
	type Target = stack::AStack<T,(u32,Option<Name>)>;
	fn deref(&self) -> &Self::Target { &self.0 }
}


impl<E: Debug+Clone+Eq+Hash+'static> From<AtTail<E>> for RazTree<E> {
	// we build this tree right to left
	// note that the left branch _cannot_ have
	// the same level as its parent
	// while the right branch can.
	// TODO: reimplement using (a new) peek_meta() to avoid half the code
	fn from(tailstack: AtTail<E>) -> Self {
		let AtTail(mut stack) = tailstack;
		fn from_stack<E: Debug+Clone+Eq+Hash+'static>(
			stack: &mut stack::AStack<E,(u32,Option<Name>)>,
			right_tree: ltree::Tree<TreeData<E>>,
			mid_level: u32,
			mid_name: Option<Name>,
			top_level: u32
		)	-> (ltree::Tree<TreeData<E>>, Option<u32>, Option<Name>) {
			match stack.next_archive() {
				None => unreachable!(), // if we have a level there will be more data
				Some((list,meta)) => { match meta {
					None => (bin(leaf(list,None), mid_level, mid_name, right_tree), None, None),
					Some((next_level,next_name)) => {
						if next_level >= top_level {
							let tree = bin(leaf(list,None),mid_level,mid_name,right_tree);
							(tree, Some(next_level),next_name)
						} else if next_level >= mid_level {
							let tree = bin(leaf(list,None),mid_level,mid_name,right_tree);
							return from_stack(stack, tree, next_level, next_name, top_level)
						} else {
							let (left_tree, left_level, left_name) = from_stack(
								stack, leaf(list,None), next_level, next_name, mid_level
							);
							let tree = bin(left_tree, mid_level, mid_name, right_tree);
							match left_level {
								None => (tree, None, None),
								Some(left_level) => {
									if left_level > top_level {
										(tree, Some(left_level), left_name)
									} else {
										return from_stack(stack, tree, left_level, left_name, top_level)
									}
								}
							}
						}
					}
				}}
			}
		}
		let (level, name, first_tree) = match stack.next_archive() {
			None => return RazTree{count: 0, tree: None},
			Some((list, meta)) => { match meta {
				None => return RazTree{count: list.len(), tree: Some(leaf(list,None))},
				Some((lev,n)) => (lev, n, leaf(list,None))
			}}
		};
		let (t,l,n) = from_stack(&mut stack, first_tree, level, name, u32::max_value());
		assert!(l.is_none());
		assert!(n.is_none());
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
	use adapton::engine::*;

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
  fn test_fold() {
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
  	assert!(cursor.up());
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
  	stack.archive((3, Some(name_of_usize(3))));
  	stack.push(3);
  	stack.push(4);
  	stack.archive((1, Some(name_of_usize(1))));
  	stack.push(5);
  	stack.push(6);
  	stack.archive((2, Some(name_of_usize(2))));
  	stack.push(7);
  	stack.push(8);
  	stack.archive((5, Some(name_of_usize(5))));
  	stack.push(9);
  	stack.push(10);
  	stack.archive((4, Some(name_of_usize(4))));
  	stack.push(11);
  	stack.push(12);
  	let raz = RazTree::from(AtTail(stack));

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
  	assert!(cursor.up());
  	assert!(cursor.down_right());
  	assert!(cursor.down_left());
  	assert!(cursor.down_left());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![3,4], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up());
  	assert!(cursor.down_right());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![5,6], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up());
  	assert!(cursor.up());
  	assert!(cursor.down_right());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![7,8], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up());
  	assert!(cursor.up());
  	assert!(cursor.up());
  	assert!(cursor.down_right());
  	assert!(cursor.down_right());
  	match cursor.peek() {
  		Some(TreeData::Leaf(ref v)) => {
  			assert_eq!(vec![11,12], **v);
  		},
  		_ => panic!("Wrong data")
  	}
  	assert!(cursor.up());
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
  	assert_eq!(Some(11), r.next());
  	assert_eq!(Some(12), r.next());
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

}
