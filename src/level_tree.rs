//! a persistent, cannonical tree that keeps track of the
//! "level" of each of its branches.
//!
//! Levels help maintain a cannonical structure that improves
//! some algorithms. All trees with the same levels will
//! have the same structure regaurdless of data or order of
//! operations.

use std::ops::Deref;
use std::rc::Rc;
use rand::Rng;
use pat::AsPattern;

/// A persistent tree with stable, internally defined structure
pub struct Tree<E> {
	level: u32,
	link: Rc<TreeNode<E>>
}
struct TreeNode<E>{
	data: E,
	l_branch: Option<Tree<E>>,
	r_branch: Option<Tree<E>>
}

impl<E> Tree<E> {
	/// build a new tree from components, return None if levels are inconsistent
	pub fn new(
		level: u32,
		data: E,
		l_branch: Option<Tree<E>>,
		r_branch: Option<Tree<E>>
	) -> Option<Tree<E>> {
		let target_level = level;
		//check level
		if let Some(Tree{level, ..}) = l_branch {
			if level > target_level { return None }
		}
		if let Some(Tree{level, ..}) = r_branch {
			if level >= target_level { return None }
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

	/// peek at the level of the root of this tree
	pub fn level(&self) -> u32 { self.level }

	/// obtain the left subtree if it exists
	pub fn l_tree(&self) -> Option<Tree<E>> { (*self.link).l_branch.clone() }

	/// obtain the right subtree if it exists
	pub fn r_tree(&self) -> Option<Tree<E>> { (*self.link).r_branch.clone() }

	/// peek at the data contained at the top node of the tree
	///
	/// this functionality is also available through Deref
	pub fn peek(&self) -> &E { &(*self.link).data }

	pub fn fold_up<R,F>(&self, node_calc: &mut F) -> R
	where
		F: FnMut(Option<R>,&E,Option<R>) -> R
	{
		match *self.link { TreeNode{ ref data, ref l_branch, ref r_branch } => {
			let l = l_branch.as_ref().map(|t| t.fold_up(node_calc));
			let r = r_branch.as_ref().map(|t| t.fold_up(node_calc));
			node_calc(l, data, r)
		}}
	}
}

/// Use good_levels to verify level consistancy when debugging
///
/// This is an O(n) operation, so it shouldn't be used in release mode
///
/// ```
/// use pmfp_collections::level_tree::{self,Tree};
///
/// let tree = Tree::new(4,(),None,Tree::new(1,(),None,None)).unwrap();
/// debug_assert!(level_tree::good_levels(&tree),"this section of code has a problem");
/// ```
///
/// checks that the levels of the tree follow the convention
/// of non-increasing to the left branch and decreasing to the
/// right branch
///
/// also prints the levels of the failing trees and branchs
pub fn good_levels<E>(tree: &Tree<E>) -> bool {
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
impl<E> Deref for Tree<E> {
	type Target = E;
	fn deref(&self) -> &E {
		self.peek()
	}
}

impl<E> Clone for Tree<E> {
	fn clone(&self) -> Self {
		Tree{level: self.level, link: self.link.clone()}
	}
}

/// deconstruction pattern
///
/// experimental deconstruction api to use with AsPattern trait
pub enum T<E> {
	Owned{level: u32, l_branch: Option<Tree<E>>, data: E, r_branch: Option<Tree<E>>},
	Shared(Tree<E>),
}

impl<E> AsPattern<T<E>> for Tree<E> {
	fn pat(self) -> T<E> {
		match Rc::try_unwrap(self.link) {
			Ok(TreeNode{data,l_branch,r_branch}) => {
				T::Owned{level: self.level, l_branch: l_branch, data: data, r_branch: r_branch}
			}
			Err(t) => T::Shared(Tree{level: self.level, link: t})
		}
	}
}

/// generate a random level appropriate for a balanced binary tree
///
/// uses a negative binomial distribution, equivalent to the
/// height of nodes (root is highest) in a balanced binary tree.
/// 
/// this will never generate a 0, reserving it for potential
/// use in tree leaves
pub fn gen_branch_level<R:Rng>(mut rng: R) -> u32 {
	let num = rng.gen::<u32>();
	(num << 1).trailing_zeros() as u32
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
		assert_eq!(21, sum);
		assert_eq!(5, depth);
		assert_eq!(true, in_order);
	}
}

