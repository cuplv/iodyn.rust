//! Split Binary Tree Cursor
//! - a cursor within a persistent, ordered, binary tree
//! - optimised for splitting and combining trees at the cursor in a cannonical way
//! - uses non-increasing levels for each subtree to maintain cannonical form
//! - in the general case the most efficent levels will be drawn from 
//!   a negative binomial distribution

use std::mem;
use std::rc::Rc;
use std::intrinsics;
use rand;
use pat::AsPattern;

/// A persistant tree, for use with a cursor. This 
/// tree is as general as possible to be of arbitrary use.
pub struct Tree<E>(TreeLink<E>);
/// A tree with the cursor here needs "levels", which are
/// non-decreasing from the root of the tree to the leaves.
pub type Level = usize;

/// Generic tree for use with the tree cursor
impl<E> Tree<E> {
	/// create an empty branch
	pub fn empty() -> Self { Tree(None) }
	/// build a new tree from components
	pub fn new(level: Level, element: E, left_branch: Tree<E>, right_branch: Tree<E>) -> Tree<E> {
		let Tree(l) = left_branch;
		let Tree(r) = right_branch;
		Tree(Some((level,Rc::new(TreeNode{data: element, l_branch: l, r_branch: r}))))
	}
	/// whether the tree contains any structure
	pub fn is_empty(&self) -> bool {
		match *self {
			Tree(None) => true,
			_ => false
		}
	}
	/// peek at the level of the root of this tree, an empty tree has level 0
	pub fn level(&self) -> Level {
		let Tree(ref t) = *self;
		match *t {
			None => 0,
			Some((lev,_)) => lev,
		}
	}
	/// peek at the data contained at the top node of the tree
	pub fn peek(&self) -> Option<&E> {
		let Tree(ref t) = *self;
		link_peek(t)
	}

	pub fn fold_up<R,F>(&self, node_calc: &mut F) -> Option<R>
	where
		F: FnMut(Option<R>,&E,Option<R>) -> R
	{
		let Tree(ref tree) = *self;
		link_fold_up(tree, node_calc)
	}

	/// for debugging, this is an O(n) operation
	///
	/// ```
	/// use pmfp_collections::split_btree_cursor::Tree;
	///
	/// let tree = Tree::new(4,(),Tree::empty(),Tree::new(1,(),Tree::empty(),Tree::empty()));
	/// debug_assert!(tree.good_levels(),"this section of code has a problem");
	/// ```
	///
	/// checks that the levels of the tree follow the convention
	/// of non-increasing to the left branch and decreasing to the
	/// right branch
	///
	/// also prints the levels of the first failing tree and branches
	pub fn good_levels(&self) -> bool {
		let Tree(ref tree) = *self;
		if let &Some((lev,ref t)) = tree {
			if lev == 0 { return true }
			let l = Tree(t.l_branch.clone());
			let r = Tree(t.r_branch.clone());
			if l.level() < lev && r.level() <= lev {
				l.good_levels() && r.good_levels()
			} else {
				println!("l: {:?}, t:{:?}, r:{:?}", l.level(),lev,r.level());
				false
			}
		} else { true }
	}

}
impl<E> Clone for Tree<E> {
	fn clone(&self) -> Self {
		let Tree(ref t) = *self;
		Tree(t.clone())
	}
}

/// deconstruction pattern
///
/// experimental deconstruction api to use with AsPattern trait
pub enum T<E> {
	None,
	Take(Level, Tree<E>, E, Tree<E>),
	Shared(Tree<E>),
}


struct TreeNode<E>{
	data: E,
	l_branch: TreeLink<E>,
	r_branch: TreeLink<E>
}
type TreeLink<E> = Option<(Level,Rc<TreeNode<E>>)>;
// Nominal Adapton design: type TreeLink<E> = Option<(Level,Option<Name>,Art<TreeNode<E>>)>; 
fn link_peek<E>(link: &TreeLink<E>) -> Option<&E>{
	link.as_ref().map(|&(_,ref node)| &node.data)
}
fn link_fold_up<E,R,F>(link: &TreeLink<E>, node_calc: &mut F) -> Option<R>
where
	F: FnMut(Option<R>,&E,Option<R>) -> R
{
	match *link {
		None => None,
		Some((_,ref t)) => match **t { TreeNode{ ref data, ref l_branch, ref r_branch } => {
			let l = link_fold_up(l_branch,node_calc);
			let r = link_fold_up(r_branch,node_calc);
			Some(node_calc(l, data, r))
		}}
	}
}


impl<E> AsPattern<T<E>> for Tree<E> {
	fn pat(self) -> T<E> {
		let Tree(node) = self;
		match node {
			None => T::None,
			Some((level,n)) =>  match Rc::try_unwrap(n) {
				Ok(TreeNode{data,l_branch,r_branch}) => {
					T::Take(level, Tree(l_branch), data, Tree(r_branch))
				}
				Err(n) => T::Shared(Tree(Some((level,n))))
			}
		}
	}
}

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
	l_forest: Vec<(bool,TreeLink<E>)>,
	tree: TreeLink<E>,
	r_forest: Vec<(bool,TreeLink<E>)>,
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

/// generate a random level appropriate for a balanced binary tree
///
/// uses a negative binomial distribution, equivalent to the height of
/// nodes (root is highest) in a balanced binary tree.
pub fn gen_level() -> Level {
	let num = rand::random::<usize>();
	let bits = unsafe{ intrinsics::ctlz(num)};
	bits+1
}

impl<E: TreeUpdate> From<Tree<E>> for Cursor<E> {
	fn from(tree: Tree<E>) -> Self {
		let Tree(tree) = tree;
		Cursor{
			dirty: false,
			l_forest: Vec::with_capacity(50),
			tree: tree,
			r_forest: Vec::with_capacity(50),
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
	pub fn with_depth(capacity: usize) -> Self {
		Cursor{
			dirty: false,
			l_forest: Vec::with_capacity(capacity),
			tree: None,
			r_forest: Vec::with_capacity(capacity),
		}
	}

	/// Returns the node the cursor is focused on as a tree, plus two
	/// cursors containing every node to the left and right, focused
	/// on at the two branches of the returned tree
	pub fn split(self) -> (Cursor<E>, Tree<E>, Cursor<E>) {
		let (l_tree,r_tree) = match self.tree {
			None => (None, None),
			Some((_,ref t)) => match **t { TreeNode{ ref l_branch, ref r_branch, ..} =>
				(l_branch.clone(), r_branch.clone())
			}
		};
		(
			Cursor{
				dirty: true,
				l_forest: self.l_forest,
				tree: l_tree,
				r_forest: Vec::with_capacity(50),
			},
			Tree(self.tree),
			Cursor{
				dirty: true,
				l_forest: Vec::with_capacity(50),
				tree: r_tree,
				r_forest: self.r_forest,
			},
		)
	}

	/// makes a new cursor at the given data, between the trees of the other cursors
	///
	/// The `update()` method of the data type will be called, with the `data`
	/// parameter passed here as the `old_data` to that method (along with joined branches).
	pub fn join(mut l_cursor: Self, level: Level, data: E, mut r_cursor: Self) -> Self {
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
		let tree = Some((level,Rc::new(TreeNode{
			data: E::update(link_peek(&l_cursor.tree), &data, link_peek(&r_cursor.tree)),
			l_branch: l_cursor.tree.clone(),
			r_branch: r_cursor.tree.clone(),
		})));
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
	pub fn at_tree(&self) -> Tree<E> {
		Tree(self.tree.clone())
	}

	/// copies the left branch of the focused node
	pub fn left_tree(&self) -> Option<Tree<E>> {
		self.tree.as_ref().map(|&(_, ref t)| Tree(t.l_branch.clone()))
	}
	/// copies the right branch of the focused node
	pub fn right_tree(&self) -> Option<Tree<E>> {
		self.tree.as_ref().map(|&(_, ref t)| Tree(t.r_branch.clone()))
	}

	/// peek at the data of the focused tree node
	pub fn peek(&self) -> Option<&E> {
		link_peek(&self.tree)
	}

	/// peek at the level of the focused tree node
	pub fn peek_level(&self) -> Option<Level> {
		self.tree.as_ref().map(|&(lev, _)| lev)
	}

	/// peek at the level of the next upper node that
	/// is to the left of this branch, even if its not
	/// directly above
	fn up_left_level(&self) -> Option<Level> {
		match self.l_forest.last() {
			None => None,
			Some(&(_,Some((lev,_)))) => Some(lev),
			_ => unreachable!(),
		}
	}
	/// peek at the level of the next upper node that
	/// is to the right of this branch, even if its not
	/// directly above
	fn up_right_level(&self) -> Option<Level> {
		match self.r_forest.last() {
			None => None,
			Some(&(_,Some((lev,_)))) => Some(lev),
			_ => unreachable!(),
		}
	}

	/// move the cursor into the left branch, returning true if successful
	/// use the `Force` enum to determine the type of movement
	pub fn down_left_force(&mut self, force: Force) -> bool {
		let new_tree = match self.tree {
			None => return false,
			Some((_,ref t)) => {
				if force == Force::No && t.l_branch.is_none() { return false }
				t.l_branch.clone()
			}
		};
		let old_tree = mem::replace(&mut self.tree, new_tree);
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
			Some((_,ref t)) => {
				if force == Force::No && t.r_branch.is_none() { return false }
				t.r_branch.clone()
			}
		};
		let old_tree = mem::replace(&mut self.tree, new_tree);
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
			(Some(&(_,Some((l_level,_)))), Some(&(_,Some((r_level,_))))) if r_level > l_level => true,
			_ => false,
		};
		if to_left {
			if let Some((dirty, Some((lev,t)))) = self.l_forest.pop() {
				if self.dirty == true {
					match *t { TreeNode{ref data, ref l_branch, ..} => {
						self.tree = Some((lev,Rc::new(TreeNode{
							data: E::update(link_peek(l_branch), data, link_peek(&self.tree)),
							l_branch: l_branch.clone(),
							r_branch: self.tree.take(),
						})));
					}}
				} else { self.dirty = dirty; self.tree = Some((lev,t)) }
			} else { panic!("up: empty left forest item"); }
		} else { // right side
			if let Some((dirty, Some((lev,t)))) = self.r_forest.pop() {
				if self.dirty == true {
					match *t { TreeNode{ref data, ref r_branch, ..} => {
						self.tree = Some((lev,Rc::new(TreeNode{
							data: E::update(link_peek(&self.tree), data, link_peek(r_branch)),
							l_branch: self.tree.take(),
							r_branch: r_branch.clone(),
						})));
					}}
				} else { self.dirty = dirty; self.tree = Some((lev,t)) }
			} else { panic!("up: empty right forest item"); }
		}
		return true;
	}

}

#[cfg(test)]
mod tests {
	use super::*;

  #[test]
  fn test_fold_up() {
  	use std::cmp::max;
		let t = 
		Tree::new(5,None,
			Tree::new(3,None,
				Tree::new(0,Some(1),Tree::empty(),Tree::empty()),
				Tree::new(2,None,
					Tree::new(1,None,
						Tree::new(0,Some(2),Tree::empty(),Tree::empty()),
						Tree::new(0,Some(3),Tree::empty(),Tree::empty()),
					),
					Tree::new(0,Some(4),Tree::empty(),Tree::empty()),
				)
			),
			Tree::new(4,None,
				Tree::new(0,Some(5),Tree::empty(),Tree::empty()),
				Tree::new(0,Some(6),Tree::empty(),Tree::empty()),
			)
		);
		let sum = t.fold_up(&mut|l,c,r| {
			l.unwrap_or(0) + c.unwrap_or(0) + r.unwrap_or(0)
		});
		let depth = t.fold_up(&mut|l,c,r| {
			match *c {
				None => max(l.unwrap(),r.unwrap()) + 1,
				Some(_) => 1,
			}
		});
		let in_order = t.fold_up(&mut|l,c,r|{
			match *c {
				None => l.unwrap() >= r.unwrap(),
				Some(_) => true,
			}
		});
		assert_eq!(Some(21), sum);
		assert_eq!(Some(5), depth);
		assert_eq!(Some(true), in_order);
	}
}


