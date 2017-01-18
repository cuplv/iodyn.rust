//! Gauged RAZ - random access sequence
//! - cursor access in low-const O(1) time
//! - arbirary access in O(log n) time
//! - combines tree cursor with stack

use std::rc::Rc;
use std::ops::Deref;

use split_btree_cursor as tree;
use archive_stack as stack;

/// Random access zipper
///
/// A cursor into a sequence, optimised for moving to 
/// arbitrary points in the sequence
#[derive(Clone)]
pub struct Raz<E: Clone> {
	length: usize,
	l_forest: tree::Cursor<TreeData<E>>,
	l_stack: stack::AStack<E,tree::Level>,
	r_stack: stack::AStack<E,tree::Level>,
	r_forest: tree::Cursor<TreeData<E>>,
}

/// The data stored in the tree structure of the RAZ.
/// Currently public for testing purposes
#[doc(hidden)]
pub enum TreeData<E> {
	Branch{l_count: usize, r_count: usize},
	Leaf(Rc<Vec<E>>),
}

fn count<E>(td: Option<&TreeData<E>>) -> usize {
	match td {
		None => 0,
		Some(&TreeData::Branch{l_count,r_count}) => l_count + r_count,
		Some(&TreeData::Leaf(ref vec)) => vec.len(),
	}
}

impl<E> tree::TreeUpdate for TreeData<E> {
	#[allow(unused_variables)]
	fn update(l_branch: Option<&Self>, old_data: &Self, r_branch: Option<&Self>) -> Self {
		TreeData::Branch{l_count: count(l_branch), r_count: count(r_branch)}
	}
}

/// Tree form of a RAZ
///
/// used between refocusing, and for running global algorithms
#[derive(Clone)]
pub struct RazTree<E: Clone>{count: usize, tree: tree::Tree<TreeData<E>>}

impl<E: Clone> Deref for RazTree<E> {
	type Target = tree::Tree<TreeData<E>>;
	fn deref(&self) -> &Self::Target {
		&self.tree
	}
}

impl<E:Clone> RazTree<E> {
	/// the number if items in the sequence
	pub fn len(&self) -> usize {self.count}

	/// Runs an binary function over the sequence data
	///
	/// This is calculated from data in leaves of a tree structure,
	/// so the operation must be associative. Returns None if there
	/// are no elements.
	pub fn fold_up<I,R,B>(&self, mut init: I, mut bin: B) -> Option<R>
		where I: FnMut(&E) -> R, B: FnMut(R,R) -> R
	{
		if self.count == 0 { return None }
		self.tree.fold_up(&mut |l,c,r|{
			match *c {
				TreeData::Leaf(ref vec) => {
					let mut iter = vec.iter().map(|elm|init(elm));
					let first = iter.next().expect("leaf with empty vec");
					iter.fold(first, &mut bin)
				},
				TreeData::Branch{..} => { match (l,r) {
					(None, None) => panic!("branch with no data"),
					(Some(r),None) | (None, Some(r)) => r,
					(Some(r1),Some(r2)) => bin(r1,r2),
				}},
			}
		})
	}

	/// focus on a location in the sequence to begin editing.
	///
	/// `0` is before the first element. This will return `None` if
	/// `pos` is larger than the number of elements.
	pub fn focus(self, mut pos: usize) -> Option<Raz<E>> {
		match self { RazTree{count,tree} => {
			if tree.is_empty() {
				return Some(Raz{
					length: 0,
					l_forest: tree::Cursor::new(),
					l_stack: stack::AStack::with_capacity(500),
					r_stack: stack::AStack::with_capacity(500),
					r_forest: tree::Cursor::new(),
				})
			}
			if count < pos { return None };
			// step 1: find location with cursor
			let mut cursor = tree::Cursor::from(tree);
			while let TreeData::Branch{l_count, ..} = *cursor.peek().unwrap() {
				if pos <= l_count {
					assert!(cursor.down_left());
				} else {
					pos -= l_count;
					assert!(cursor.down_right());
				}
			}
			// step 2: extract and copy data
			let (l_cursor, tree, r_cursor) = cursor.split();
			let (l_slice,r_slice) = match *tree.peek().unwrap() {
				TreeData::Branch{..} => unreachable!(),
				TreeData::Leaf(ref vec_ref) => vec_ref.split_at(pos)
			};
			let mut l_gstack = stack::AStack::with_capacity(500);
			let mut r_gstack = stack::AStack::with_capacity(500);
			l_gstack.extend(l_slice);
			r_gstack.extend_rev(r_slice);
			// step 3: integrate
			Some(Raz{
				length: count,
				l_forest: l_cursor,
				l_stack: l_gstack,
				r_stack: r_gstack,
				r_forest: r_cursor,
			})
		}}
	}

}

impl<E:Clone> Raz<E> {
	/// Create a new RAZ, for an empty sequence
	pub fn new() -> Raz<E> {
		Raz{
			length: 0,
			l_forest: tree::Cursor::new(),
			l_stack: stack::AStack::with_capacity(500),
			r_stack: stack::AStack::with_capacity(500),
			r_forest: tree::Cursor::new(),
		}
	}
	/// unfocus the RAZ before refocusing on a new location
	/// in the sequence.
	pub fn unfocus(mut self) -> RazTree<E> {
		let mut l_lev = None;
		let mut r_lev = None;
		// step 1: reconstruct local array from stack
		let l_vec = if let Some((vec,lev)) = self.l_stack.next_archive() {
			l_lev = lev;
			if vec.len() > 0 {Some(vec)} else {None}
		} else { None };
		let r_vec = if let Some((vec,lev)) = self.r_stack.next_archive() {
			r_lev = lev;
			if vec.len() > 0 {Some(vec)} else {None}
		} else { None };
		let vec = match (self.l_stack.is_empty(), l_vec, r_vec, self.r_stack.is_empty()) {
			(_,Some(v),None,_) => Some(v),
			(_,None,Some(mut v),_) => { v.reverse(); Some(v) },
			(_,Some(mut lv),Some(mut rv),_) => {rv.reverse(); lv.extend(rv); Some(lv) },
			(false, None, None, _) => {
				let (v,lev) = self.l_stack.next_archive().unwrap();
				l_lev = lev;
				Some(v)
			},
			(true, None, None, false) => {
				let (mut v,lev) = self.r_stack.next_archive().unwrap();
				v.reverse();
				r_lev = lev;
				Some(v)
			},
			_ => None
		};
		// step 2: build center tree
		let tree = if let Some(v) = vec {
			// OPTIMIZE: linear algorithm
			let mut cursor = tree::Tree::new(0,TreeData::Leaf(Rc::new(v)),tree::Tree::empty(),tree::Tree::empty()).into();
			while let Some((l_vec,lev)) = self.l_stack.next_archive() {
				let l_curs = tree::Tree::new(0,TreeData::Leaf(Rc::new(l_vec)),tree::Tree::empty(),tree::Tree::empty()).into();
				let dummy = TreeData::Branch{l_count: 0, r_count: 0};
				cursor = tree::Cursor::join(l_curs,l_lev.unwrap(),dummy,cursor);
				l_lev = lev;
			}
			while let Some((mut r_vec,lev)) = self.r_stack.next_archive() {
				r_vec.reverse();
				let r_curs = tree::Tree::new(0,TreeData::Leaf(Rc::new(r_vec)),tree::Tree::empty(),tree::Tree::empty()).into();
				let dummy = TreeData::Branch{l_count: 0, r_count: 0};
				cursor = tree::Cursor::join(cursor,r_lev.unwrap(),dummy,r_curs);
				r_lev = lev;
			}
			while cursor.up() {}
			cursor.at_tree()
		} else {
			if !self.l_forest.up() {
				if !self.r_forest.up() {
					return RazTree{ count: 0, tree: tree::Tree::empty() };
				} else {
					self.r_forest.right_tree().unwrap()
				}
			} else {
				self.l_forest.left_tree().unwrap()
			}
		};
		// step 3: join with forests
		let mut join_cursor: tree::Cursor<TreeData<E>> = tree.into();
		if self.l_forest.up() {
			let lev = self.l_forest.peek_level().unwrap();
			self.l_forest.down_left_force(tree::Force::Discard);
			let dummy = TreeData::Branch{l_count: 0, r_count: 0};
			join_cursor = tree::Cursor::join(self.l_forest,lev,dummy,join_cursor);
		}
		if self.r_forest.up() {
			let lev = self.r_forest.peek_level().unwrap();
			self.r_forest.down_right_force(tree::Force::Discard);
			let dummy = TreeData::Branch{l_count: 0, r_count: 0};
			join_cursor = tree::Cursor::join(join_cursor,lev,dummy,self.r_forest);
		}
		// step 4: convert to final tree
		while join_cursor.up() {}
		let tree = join_cursor.at_tree();
		debug_assert!(tree.good_levels(), "unfocused tree has bad levels");
		RazTree{count: count(tree.peek()), tree: tree}
	}

	/// add an element to the left of the cursor
	pub fn push_left(&mut self, elm: E) {
		self.length += 1;
		self.l_stack.push(elm);
		if self.l_stack.active_len() % 200 == 0 { self.archive_left(tree::gen_level()) }
	}
	/// add an element to the right of the cursor
	pub fn push_right(&mut self, elm: E) {
		self.length += 1;
		self.r_stack.push(elm);
		if self.r_stack.active_len() % 200 == 0 { self.archive_right(tree::gen_level()) }
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
	pub fn archive_left(&mut self, level: tree::Level) {
		self.l_stack.archive(level);
	}
	/// mark the data at the right to be shared
	pub fn archive_right(&mut self, level: tree::Level) {
		self.r_stack.archive(level);
	}
	/// remove and return an element to the left of the cursor
	pub fn pop_left(&mut self) -> Option<E> {
		if self.l_stack.len() == 0 {
			if !self.l_forest.up() { return None } else {
				self.l_forest.down_left_force(tree::Force::Discard);
				while self.l_forest.down_right() {}
				match self.l_forest.peek() {
					Some(&TreeData::Leaf(ref data)) => self.l_stack.extend(&***data),
					_ => panic!("pop_left: no left tree leaf"),
				}
			}
		}
		self.length -= 1;
		self.l_stack.pop()
	}
	/// remove and return an element to the right of the cursor
	pub fn pop_right(&mut self) -> Option<E> {
		if self.r_stack.len() == 0 {
			if !self.r_forest.up() { return None } else {
				self.r_forest.down_right_force(tree::Force::Discard);
				while self.r_forest.down_left() {}
				match self.r_forest.peek() {
					Some(&TreeData::Leaf(ref data)) => self.r_stack.extend_rev(&***data),
					_ => panic!("pop_right: no right tree leaf"),
				}
			}
		}
		self.length -= 1;
		self.r_stack.pop()
	}
}

////////////////////////////////
// Zip and ZipSeq for Gauged RAZ 
////////////////////////////////

use zip::Zip;
use seqzip::{Seq, SeqZip};

impl<E: Clone> Zip<E> for Raz<E> {
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

impl<E: Clone> Seq<E, Raz<E>> for RazTree<E> {
	fn zip_to(&self, loc: usize) -> Result<Raz<E>,&str> {
		self.clone().focus(loc).ok_or("Gauged RAZ: focus out of range")
	}
}

impl<E: Clone> SeqZip<E, RazTree<E>> for Raz<E> {
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
  		tree: tree::Tree::new(5,TreeData::Branch{l_count:8, r_count: 4},
  			tree::Tree::new(3,TreeData::Branch{l_count:2, r_count: 6},
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(1,2))),tree::Tree::empty(),tree::Tree::empty()),
  				tree::Tree::new(2,TreeData::Branch{l_count:4, r_count: 2},
  					tree::Tree::new(1,TreeData::Branch{l_count:2, r_count: 2},
		  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(3,4))),tree::Tree::empty(),tree::Tree::empty()),
		  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(5,6))),tree::Tree::empty(),tree::Tree::empty()),
  					),
  					tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(7,8))),tree::Tree::empty(),tree::Tree::empty()),
  				)
  			),
  			tree::Tree::new(4,TreeData::Branch{l_count: 2, r_count: 2},
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(9,10))),tree::Tree::empty(),tree::Tree::empty()),
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(11,12))),tree::Tree::empty(),tree::Tree::empty()),
  			)
  		)
  	};
  	assert!(tree.good_levels());

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
  	r.archive_left(1);
  	r.push_right(8);
  	r.push_right(7);
  	r.archive_right(2);
  	r.push_left(5);
  	r.push_right(6);
  	t = r.unfocus();
  	r = t.focus(0).expect("focus on 0");
  	r.push_left(1);
  	r.push_left(2);
  	r.archive_left(3);
  	t = r.unfocus();

  	r = t.focus(8).expect("focus on 8");
  	r.archive_left(5);
  	r.push_left(9);
  	r.push_left(10);
  	r.push_right(12);
  	r.push_right(11);
  	r.archive_right(4);
  	t = r.unfocus();

  	assert!(t.good_levels());

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
  		tree: tree::Tree::new(5,TreeData::Branch{l_count:8, r_count: 4},
  			tree::Tree::new(3,TreeData::Branch{l_count:2, r_count: 6},
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(1,2))),tree::Tree::empty(),tree::Tree::empty()),
  				tree::Tree::new(2,TreeData::Branch{l_count:4, r_count: 2},
  					tree::Tree::new(1,TreeData::Branch{l_count:2, r_count: 2},
		  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(3,4))),tree::Tree::empty(),tree::Tree::empty()),
		  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(5,6))),tree::Tree::empty(),tree::Tree::empty()),
  					),
  					tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(7,8))),tree::Tree::empty(),tree::Tree::empty()),
  				)
  			),
  			tree::Tree::new(4,TreeData::Branch{l_count: 2, r_count: 2},
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(9,10))),tree::Tree::empty(),tree::Tree::empty()),
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(11,12))),tree::Tree::empty(),tree::Tree::empty()),
  			)
  		)
  	};
  	assert!(tree.good_levels());

  	let max = tree.fold_up(|e|{*e},|e1,e2|{::std::cmp::max(e1,e2)}).unwrap();
  	assert_eq!(12, max);

  	let sum = tree.fold_up(|e|{*e},|e1,e2|{e1+e2}).unwrap_or(0);
  	let iter_sum: usize = (1..13).sum();
  	assert_eq!(iter_sum, sum);

  	#[derive(PartialEq,Eq,Debug)]
  	enum EO {Even,Odd}
  	let even_odd = tree.fold_up(
  		|e| if *e % 2 == 0 {EO::Even} else {EO::Odd} ,
  		|e1,e2| if e1 == e2 {EO::Even} else {EO::Odd}
  	).unwrap();
  	assert_eq!(EO::Even, even_odd);

  }
}