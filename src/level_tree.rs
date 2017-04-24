//! a persistent, cannonical tree that keeps track of the
//! "level" of each of its branches.
//!
//! Levels help maintain a cannonical structure that improves
//! some algorithms. All trees with the same levels will
//! have the same structure regaurdless of data or order of
//! operations.

use std::ops::Deref;
use std::rc::Rc;

use trees::{BinTree,LevelTree,Level};

/// A persistent tree with stable, internally defined structure
#[derive(Debug,PartialEq,Eq)]
pub struct Tree<L: Level,E> {
	level: L,
	link: Rc<TreeNode<L,E>>
}
#[derive(Debug,PartialEq,Eq)]
struct TreeNode<L: Level,E>{
	data: E,
	l_branch: Option<Tree<L,E>>,
	r_branch: Option<Tree<L,E>>
}

impl<L: Level,E> Tree<L,E> {
	/// build a new tree from components, return None if levels are inconsistent
	pub fn new(
		level: L,
		data: E,
		l_branch: Option<Tree<L,E>>,
		r_branch: Option<Tree<L,E>>
	) -> Option<Tree<L,E>> {
		let target_level = level;
		//check level
		if let Some(Tree{level, ..}) = l_branch {
			if level >= target_level { return None }
		}
		if let Some(Tree{level, ..}) = r_branch {
			if level > target_level { return None }
		}
		// structure the data
		Some(Tree{
			level: level,
			link: Rc::new(TreeNode{
				data: data,
				l_branch: l_branch,
				r_branch: r_branch
			})
		})
	}

	pub fn map<R,F>(&self, map_val: &mut F) -> Tree<L,R>
	where
		F: FnMut(&E) -> R
	{
		match *self.link { TreeNode{ ref data, ref l_branch, ref r_branch } => {
			let l = l_branch.as_ref().map(|t| t.map(map_val));
			let r = r_branch.as_ref().map(|t| t.map(map_val));
			Tree::new(self.level,map_val(data),l,r).unwrap()
		}}
	}

}

impl<L: Level,E> LevelTree<L,E> for Tree<L,E> {
	fn lev_make(
		level: L,
		data: E,
		l_branch: Option<Self>,
		r_branch: Option<Self>
	) -> Option<Self> {
		Tree::new(level,data,l_branch,r_branch)
	}
	/// peek at the level of the root of this tree
	fn level(&self) -> L { self.level }
}

impl<L: Level, E> BinTree<E> for Tree<L,E> {
	/// obtain the left subtree if it exists
	fn l_tree(&self) -> Option<Tree<L,E>> { (*self.link).l_branch.clone() }

	/// obtain the right subtree if it exists
	fn r_tree(&self) -> Option<Tree<L,E>> { (*self.link).r_branch.clone() }

	/// peek at the data contained at the top node of the tree
	///
	/// this functionality is also available through Deref
	fn peek(&self) -> &E { &(*self.link).data }

	fn fold_up<R,F>(&self, node_calc: &mut F) -> R
	where
		F: FnMut(&E,Option<R>,Option<R>) -> R
	{
		match *self.link { TreeNode{ ref data, ref l_branch, ref r_branch } => {
			let l = l_branch.as_ref().map(|t| t.fold_up(node_calc));
			let r = r_branch.as_ref().map(|t| t.fold_up(node_calc));
			node_calc(data, l, r)
		}}
	}

}

/// Use good_levels to verify level consistancy when debugging
///
/// This is an O(n) operation, so it shouldn't be used in release mode
///
/// ```
/// use iodyn::level_tree::{self,Tree};
/// use iodyn::trees::NegBin;
///
/// let tree = Tree::new(NegBin(4),(),None,Tree::new(NegBin(1),(),None,None)).unwrap();
/// debug_assert!(level_tree::good_levels(&tree),"this section of code has a problem");
/// ```
///
/// checks that the levels of the tree follow the convention
/// of non-increasing to the left branch and decreasing to the
/// right branch
///
/// also prints the levels of the failing trees and branchs
pub fn good_levels<L: Level, E>(tree: &Tree<L,E>) -> bool {
	let mut good = true;
	if let Some(ref t) = (*tree.link).l_branch {
		if t.level > tree.level {
			println!("Tree with level {:?} has left branch with level {:?}", tree.level, t.level);
			good = false;
		}
		if !good_levels(t) { good = false }
	}
	if let Some(ref t) = (*tree.link).r_branch {
		if t.level >= tree.level {
			println!("Tree with level {:?} has right branch with level {:?}", tree.level, t.level);
			good = false;
		}
		if !good_levels(t) { good = false }
	}
	good
}

/// dereference to the data stored in this tree node
impl<L: Level,E> Deref for Tree<L,E> {
	type Target = E;
	fn deref(&self) -> &E {
		self.peek()
	}
}

impl<L: Level,E> Clone for Tree<L,E> {
	fn clone(&self) -> Self {
		Tree{level: self.level, link: self.link.clone()}
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
				Tree::new(0,Some(1),None,None),
				Tree::new(2,None,
					Tree::new(1,None,
						Tree::new(0,Some(2),None,None),
						Tree::new(0,Some(3),None,None),
					),
					Tree::new(0,Some(4),None,None),
				)
			),
			Tree::new(4,None,
				Tree::new(0,Some(5),None,None),
				Tree::new(0,Some(6),None,None),
			)
		).unwrap();
		let sum = t.fold_up(&mut|d,l,r| {
			l.unwrap_or(0) + d.unwrap_or(0) + r.unwrap_or(0)
		});
		let depth = t.fold_up(&mut|d,l,r| {
			match *d {
				None => max(l.unwrap(),r.unwrap()) + 1,
				Some(_) => 1,
			}
		});
		let in_order = t.fold_up(&mut|d,l,r|{
			match *d {
				None => l.unwrap() >= r.unwrap(),
				Some(_) => true,
			}
		});
		assert_eq!(21, sum);
		assert_eq!(5, depth);
		assert_eq!(true, in_order);
	}

  #[test]
  fn test_map() {
		let t = 
		Tree::new(5,None,
			Tree::new(3,None,
				Tree::new(0,Some(1),None,None),
				Tree::new(2,None,
					Tree::new(1,None,
						Tree::new(0,Some(2),None,None),
						Tree::new(0,Some(3),None,None),
					),
					Tree::new(0,Some(4),None,None),
				)
			),
			Tree::new(4,None,
				Tree::new(0,Some(5),None,None),
				Tree::new(0,Some(6),None,None),
			)
		).unwrap();
		let tree_plus1 = t.map(&mut|d| { d.map(|n|n+1) });
		let leaf = tree_plus1.l_tree().unwrap().r_tree().unwrap().l_tree().unwrap().r_tree().unwrap();
		assert_eq!(&Some(4), leaf.peek());

		let sum = tree_plus1.fold_up(&mut|d,l,r| {
			l.unwrap_or(0) + d.unwrap_or(0) + r.unwrap_or(0)
		});
		assert_eq!(27, sum);
	}
}


