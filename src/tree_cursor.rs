//! Tree Cursor for `level_tree`
//! - a cursor within a persistent, ordered, binary tree
//! - optimised for splitting and combining trees at the cursor in a cannonical way
//! - uses non-increasing levels for each subtree to maintain cannonical form
//! - in the general case the most efficent levels will be drawn from 
//!   a negative binomial distribution


use std::mem;
pub use level_tree::{Tree, gen_branch_level as gen_level};

/// tree cursor, centered on a node of the underlying persistent tree
///
/// A cursor allows exploration of the underlying tree by following links
/// to branches or up towards root. This structure is optimised 
/// for splitting and combining trees in a cannonical way based on
/// the levels of the nodes; that is, trees with the same levels will have
/// the same structure, regaurdless of order of operations.
/// 
/// Many operations allow structural mutation of the underlying tree.
pub struct Cursor<E: TreeUpdate> {
	dirty: bool,
	// dirty flag, containing tree
	l_forest: Vec<(bool,Tree<E>)>,
	tree: Option<Tree<E>>,
	r_forest: Vec<(bool,Tree<E>)>,
}
impl<E: TreeUpdate> Clone for Cursor<E> {
	fn clone(&self) -> Self {
		Cursor {
			dirty: self.dirty,
			l_forest: self.l_forest.clone(),
			tree: self.tree.clone(),
			r_forest: self.r_forest.clone(),
		}
	}
}

/// Used for updating data when the tree is mutated
///
/// When the full tree is reconstructed on demand as the user
/// moves up to the root, new persistent branches must be constructed.
/// This trait allows the user to define how data is reconstructed.
pub trait TreeUpdate {
	/// This method provides references to the (potentially) newly defined left and
	/// right branches of a tree node, along with the old data in that node.
	/// For example, read size from left and right to get the new size of the branch,
	/// or copy the old data without modification for the new branch.
	fn update(l_branch: Option<&Self>, old_data: &Self, r_branch: Option<&Self>) -> Self;
}
/// marker that allows a default implementation of TreeUpdate if the data is also `Clone`
///
/// this will simply clone the old data to be used in the new node
pub trait DeriveTreeUpdate{}
impl<E: DeriveTreeUpdate + Clone> TreeUpdate for E {
	#[allow(unused_variables)]
	fn update(l_branch: Option<&Self>, old_data: &Self, r_branch: Option<&Self>) -> Self { old_data.clone() }
}

/// cursor movement qualifier
///
/// There are a few options for cursor movement to branches:
/// - Force::No, moves to the branch if it is not empty
/// - Force::Yes, forces the move to an empty branch
/// - Force::Discard, moves to full or empty branchs, discarding the
///   currently active branch. This will effectively connect the upper
///   node to the lower node, bypassing the current one 
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Force {
	No,
	Yes,
	Discard,
}

fn peek_op<E>(op: &Option<Tree<E>>) -> Option<&E> {
	match *op {
		None => None,
		Some(ref t) => Some(t.peek())
	}
}

const DEFAULT_DEPTH: usize = 30;

impl<E: TreeUpdate> From<Tree<E>> for Cursor<E> {
	fn from(tree: Tree<E>) -> Self {
		Cursor{
			dirty: false,
			l_forest: Vec::with_capacity(DEFAULT_DEPTH),
			tree: Some(tree),
			r_forest: Vec::with_capacity(DEFAULT_DEPTH),
		}
	}
}

impl<E: TreeUpdate> Cursor<E> {

	/// creates a new cursor, to an empty underlying tree
	pub fn new() -> Self {
		Cursor{
			dirty: false,
			l_forest: Vec::new(),
			tree: None,
			r_forest: Vec::new(),
		}
	}
	/// creates a new cursor, with expected depth of underlying tree
	pub fn with_depth(depth: usize) -> Self {
		Cursor{
			dirty: false,
			l_forest: Vec::with_capacity(depth),
			tree: None,
			r_forest: Vec::with_capacity(depth),
		}
	}

	/// Returns the node the cursor is focused on as a tree, plus two
	/// cursors containing every node to the left and right, focused
	/// on at the two branches of the returned tree
	pub fn split(self) -> (Cursor<E>, Option<Tree<E>>, Cursor<E>) {
		let (l_tree,r_tree) = match self.tree {
			None => (None, None),
			Some(ref t) => (t.l_tree(), t.r_tree())
		};
		(
			Cursor{
				dirty: true,
				l_forest: self.l_forest,
				tree: l_tree,
				r_forest: Vec::with_capacity(DEFAULT_DEPTH),
			},
			self.tree,
			Cursor{
				dirty: true,
				l_forest: Vec::with_capacity(DEFAULT_DEPTH),
				tree: r_tree,
				r_forest: self.r_forest,
			},
		)
	}

	/// makes a new cursor at the given data, between the trees of the other cursors
	///
	/// The `update()` method of the data type will be called, with the `data`
	/// parameter passed here as the `old_data` to that method (along with joined branches).
	pub fn join(mut l_cursor: Self, level: u32, data: E, mut r_cursor: Self) -> Self {
		// step 1: remove center forests
		while !l_cursor.r_forest.is_empty() { assert!(l_cursor.up()); }
		while !r_cursor.l_forest.is_empty() { assert!(r_cursor.up()); }
		// step 2: find insertion point
		while let Some(h) = l_cursor.up_left_level() {
			if h >= level { break; }
			else { assert!(l_cursor.up()); }
		}
		while let Some(h) = l_cursor.peek_level() {
			if h < level { break; }
			else { assert!(l_cursor.down_right_force(Force::Yes)); }
		}
		while let Some(h) = r_cursor.up_right_level() {
			if h > level { break; }
			else { assert!(r_cursor.up()); }
		}
		while let Some(h) = r_cursor.peek_level() {
			if h <= level { break; }
			else { assert!(r_cursor.down_left_force(Force::Yes)); }
		}
		// step 3: build center tree
		let tree = Tree::new(
			level,
			E::update(peek_op(&l_cursor.tree), &data, peek_op(&r_cursor.tree)),
			l_cursor.tree.clone(),
			r_cursor.tree.clone(),
		);
		assert!(tree.is_some());
		// step4: join structures
		Cursor{
			dirty: true,
			l_forest: l_cursor.l_forest,
			tree: tree,
			r_forest: r_cursor.r_forest,
		}
	}

	/// copies the focused node as a tree
	///
	/// This is a persistent tree, so copies are Rc clones
	pub fn at_tree(&self) -> Option<Tree<E>> { self.tree.clone() }

	/// copies the left branch of the focused node
	pub fn left_tree(&self) -> Option<Tree<E>> {
		match self.tree { None => None, Some(ref t) => t.l_tree() }
	}
	/// copies the right branch of the focused node
	pub fn right_tree(&self) -> Option<Tree<E>> {
		match self.tree { None => None, Some(ref t) => t.r_tree() }
	}

	/// peek at the data of the focused tree node
	pub fn peek(&self) -> Option<&E> {
		peek_op(&self.tree)
	}

	/// peek at the level of the focused tree node
	pub fn peek_level(&self) -> Option<u32> {
		self.tree.as_ref().map(|t| t.level())
	}

	/// peek at the level of the next upper node that
	/// is to the left of this branch, even if its not
	/// directly above
	fn up_left_level(&self) -> Option<u32> {
		match self.l_forest.last() {
			None => None,
			Some(&(_,ref t)) => Some(t.level()),
		}
	}
	/// peek at the level of the next upper node that
	/// is to the right of this branch, even if its not
	/// directly above
	fn up_right_level(&self) -> Option<u32> {
		match self.r_forest.last() {
			None => None,
			Some(&(_,ref t)) => Some(t.level()),
		}
	}

	/// move the cursor into the left branch, returning true if successful
	/// use the `Force` enum to determine the type of movement
	pub fn down_left_force(&mut self, force: Force) -> bool {
		let new_tree = match self.tree {
			None => return false,
			Some(ref t) => { 
				let lt = t.l_tree();
				if lt.is_none() && force == Force::No { return false }
				lt
			}
		};
		let old_tree = mem::replace(&mut self.tree, new_tree).unwrap();
		if force != Force::Discard {
			self.r_forest.push((self.dirty, old_tree));
			self.dirty = false;
		} else { self.dirty = true; }
		true
	}
	/// move the cursor to the left branch, without entering an empty branch
	pub fn down_left(&mut self) -> bool { self.down_left_force(Force::No) }

	/// move the cursor into the right branch, returning true if successful
	/// use the `Force` enum to determine the type of movement
	pub fn down_right_force(&mut self, force: Force) -> bool {
		let new_tree = match self.tree {
			None => return false,
			Some(ref t) => { 
				let rt = t.r_tree();
				if rt.is_none() && force == Force::No { return false }
				rt
			}
		};
		let old_tree = mem::replace(&mut self.tree, new_tree).unwrap();
		if force != Force::Discard {
			self.l_forest.push((self.dirty, old_tree));
			self.dirty = false;
		} else { self.dirty = true; }
		true
	}
	/// move the cursor to the right branch, without entering an empty branch
	pub fn down_right(&mut self) -> bool { self.down_right_force(Force::No) }

	/// move the cursor up towards the root of the underlying persistent tree
	///
	/// If the tree has been changed, the `update()` method of the tree's data
	/// type will be called as a new persistent node is created.
	pub fn up(&mut self) -> bool {
		let to_left = match (self.l_forest.last(), self.r_forest.last()) {
			(None, None) => { return false },
			(Some(_), None) => true,
			(Some(&(_,ref lt)), Some(&(_,ref rt))) if rt.level() > lt.level() => true,
			_ => false,
		};
		if to_left {
			if let Some((dirty, upper_tree)) = self.l_forest.pop() {
				if self.dirty == true {
					let l_branch = upper_tree.l_tree();
					self.tree = Tree::new(
						upper_tree.level(),
						E::update(peek_op(&l_branch), upper_tree.peek(), peek_op(&self.tree)),
						l_branch,
						self.tree.take(),
					)
				} else { self.dirty = dirty; self.tree = Some(upper_tree) }
			} else { panic!("up: empty left forest item"); }
		} else { // right side
			if let Some((dirty, upper_tree)) = self.r_forest.pop() {
				if self.dirty == true {
					let r_branch = upper_tree.r_tree();
					self.tree = Tree::new(
						upper_tree.level(),
						E::update(peek_op(&self.tree), upper_tree.peek(), peek_op(&r_branch)),
						self.tree.take(),
						r_branch,
					)
				} else { self.dirty = dirty; self.tree = Some(upper_tree) }
			} else { panic!("up: empty right forest item"); }
		}
		return true;
	}

}

#[cfg(test)]
mod tests {
	use super::*;
	use level_tree::Tree;

	impl DeriveTreeUpdate for usize {}

  #[test]
  fn test_movement() {
		let t = 
		Tree::new(5,1,
			Tree::new(3,2,
				Tree::new(0,4,None,None),
				Tree::new(2,5,
					Tree::new(1,8,
						Tree::new(0,10,None,None),
						Tree::new(0,11,None,None),
					),
					Tree::new(0,9,None,None),
				)
			),
			Tree::new(4,3,
				Tree::new(0,6,None,None),
				Tree::new(0,7,None,None),
			)
		).unwrap();

		let mut c: Cursor<usize> = t.into();
		assert_eq!(Some(&1), c.peek());

		assert!(c.down_left());
		assert!(c.down_right());
		assert!(c.down_left());
		assert!(c.down_right());
		assert_eq!(Some(&11), c.peek());

		assert!(c.up());
		assert!(c.up());
		assert!(c.up());
		assert_eq!(Some(&2), c.peek());

		assert!(c.down_left_force(Force::Discard));
		assert_eq!(Some(&4), c.peek());

		assert!(c.up());
		assert!(c.down_right());
		assert!(c.down_right());
		assert_eq!(Some(&7), c.peek());

		assert!(!c.down_right());
		assert!(c.down_right_force(Force::Yes));
		assert_eq!(None, c.peek());
	}

	#[test]
	fn test_split_join() {
		let t = 
		Tree::new(5,1,
			Tree::new(3,2,
				Tree::new(0,4,None,None),
				Tree::new(2,5,
					Tree::new(1,8,
						Tree::new(0,10,None,None),
						Tree::new(0,11,None,None),
					),
					Tree::new(0,9,None,None),
				)
			),
			Tree::new(4,3,
				Tree::new(0,6,None,None),
				Tree::new(0,7,None,None),
			)
		).unwrap();

		let mut c: Cursor<usize> = t.into();
		assert!(c.down_left());
		let (mut lc, t, mut rc) = c.split();
		assert_eq!(Some(&4), lc.peek());
		assert_eq!(Some(&5), rc.peek());

		assert!(!lc.up());
		assert!(rc.up());
		assert_eq!(Some(&1), rc.peek());

		let t = t.unwrap();
		let mut j = Cursor::join(rc, t.level(), (*t).clone(), lc);
		assert_eq!(Some(&2), j.peek());

		assert!(j.down_left());
		assert_eq!(Some(&7), j.peek());

		assert!(j.up());
		assert!(j.up());
		assert_eq!(Some(&3), j.peek());
	}
}


