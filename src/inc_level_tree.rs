//! Temporary alteration of level_tree for incremental use
//!
//! a persistent, cannonical tree that keeps track of the
//! "level" of each of its branches.
//!
//! Levels help maintain a cannonical structure that improves
//! some algorithms. All trees with the same levels will
//! have the same structure regaurdless of data or order of
//! operations.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use rand::Rng;

use adapton::macros::*;
use adapton::engine::*;

/// A persistent tree with stable, internally defined structure
#[derive(Debug,PartialEq,Eq,Hash)]
pub struct Tree<E: 'static+Debug+Clone+Eq+Hash> {
	level: u32,
	name: Option<Name>,
	link: Art<TreeNode<E>>
}
#[derive(Debug,PartialEq,Eq,Clone,Hash)]
struct TreeNode<E: 'static+Debug+Clone+Eq+Hash>{
	data: E,
	l_branch: Option<Tree<E>>,
	r_branch: Option<Tree<E>>
}

impl<E: Debug+Clone+Eq+Hash+'static> Tree<E> {
	/// build a new tree from components, return None if levels are inconsistent
	pub fn new(
		level: u32,
		name: Option<Name>,
		data: E,
		l_branch: Option<Tree<E>>,
		r_branch: Option<Tree<E>>
	) -> Option<Tree<E>> {
		let target_level = level;
		//check level
		if let Some(Tree{level, ..}) = l_branch {
			if level >= target_level { return None }
		}
		if let Some(Tree{level, ..}) = r_branch {
			if level > target_level { return None }
		}
		// structure the data
		match name {
			Some(name) => Some(Tree{
				level: level, name: Some(name.clone()),
				link: cell(name, TreeNode{
					data: data,
					l_branch: l_branch,
					r_branch: r_branch
				})
			}),
			None => Some(Tree{
				level: level, name: None,
				link: put(TreeNode{
					data: data,
					l_branch: l_branch,
					r_branch: r_branch
				})
			}),
		}
	}

	/// peek at the level of the root of this tree
	pub fn level(&self) -> u32 { self.level }

	/// peek at the name of the root of this tree
	pub fn name(&self) -> Option<Name> { self.name.clone() }

	/// obtain the left subtree if it exists
	pub fn l_tree(&self) -> Option<Tree<E>> { force(&self.link).l_branch.clone() }

	/// obtain the right subtree if it exists
	pub fn r_tree(&self) -> Option<Tree<E>> { force(&self.link).r_branch.clone() }

	/// peek at the data contained at the top node of the tree
	///
	/// this functionality is _not_ available through Deref
	pub fn peek(&self) -> E { force(&self.link).data }

	/// memoized fold operation
	pub fn fold_up<R:Eq+Clone+Hash+Debug+'static,F>(self, node_calc: Rc<F>) -> R
  where
    F: 'static + Fn(Option<R>,E,Option<R>) -> R
  {
    match force(&self.link) { TreeNode{ data, l_branch, r_branch } => {
      let (l,r) = match self.name {
      	None => {(
      		l_branch.map(|t| t.fold_up(node_calc.clone())),
      		r_branch.map(|t| t.fold_up(node_calc.clone())),
      	)},
      	Some(name) => {
		      let (n1, n2) = name_fork(name);
      		(
			      l_branch.map(|t| memo!( n1 =>> Self::fold_up , t:t ;; f:node_calc.clone() )),
			      r_branch.map(|t| memo!( n2 =>> Self::fold_up , t:t ;; f:node_calc.clone() )),
      		)
      	}
      };
      node_calc(l, data, r)
    }}
  }
}

/// Use good_levels to verify level consistancy when debugging
///
/// This is an O(n) operation, so it shouldn't be used in release mode
///
/// ```
/// use pmfp_collections::inc_level_tree::{good_levels,Tree};
///
/// let tree = Tree::new(4,None,(),None,Tree::new(1,None,(),None,None)).unwrap();
/// debug_assert!(good_levels(&tree),"this section of code has a problem");
/// ```
///
/// checks that the levels of the tree follow the convention
/// of non-increasing to the left branch and decreasing to the
/// right branch
///
/// also prints the levels of the failing trees and branchs
pub fn good_levels<E: Debug+Clone+Eq+Hash+'static>(tree: &Tree<E>) -> bool {
	let mut good = true;
	if let Some(ref t) = force(&tree.link).l_branch {
		if t.level > tree.level {
			println!("Tree with level {:?} has left branch with level {:?}", tree.level, t.level);
			good = false;
		}
		if !good_levels(t) { good = false }
	}
	if let Some(ref t) = force(&tree.link).r_branch {
		if t.level >= tree.level {
			println!("Tree with level {:?} has right branch with level {:?}", tree.level, t.level);
			good = false;
		}
		if !good_levels(t) { good = false }
	}
	good
}

impl<E: Debug+Clone+Eq+Hash+'static> Clone for Tree<E> {
	fn clone(&self) -> Self {
		Tree{level: self.level, name: self.name.clone(), link: self.link.clone()}
	}
}

/// generate a random level appropriate for a balanced binary tree
///
/// uses a negative binomial distribution, equivalent to the
/// height of nodes (root is highest) in a balanced binary tree.
/// 
/// this will never generate a 0, reserving it for potential
/// use in tree leaves
pub fn gen_branch_level<R:Rng>(rng: &mut R) -> u32 {
	let num = rng.gen::<u64>();
	(num << 1).trailing_zeros() as u32
}

#[cfg(test)]
mod tests {
	use super::*;

  #[test]
  fn test_fold_up() {
  	use std::cmp::max;
		let t = 
		Tree::new(5, Some(name_of_usize(5)),None,
			Tree::new(3, Some(name_of_usize(3)),None,
				Tree::new(0,None,Some(1),None,None),
				Tree::new(2, Some(name_of_usize(2)),None,
					Tree::new(1, Some(name_of_usize(1)),None,
						Tree::new(0,None,Some(2),None,None),
						Tree::new(0,None,Some(3),None,None),
					),
					Tree::new(0,None,Some(4),None,None),
				)
			),
			Tree::new(4, Some(name_of_usize(4)),None,
				Tree::new(0,None,Some(5),None,None),
				Tree::new(0,None,Some(6),None,None),
			)
		).unwrap();
		let sum = t.clone().fold_up(Rc::new(|l: Option<usize>,c: Option<usize>,r: Option<usize>| {
			l.unwrap_or(0) + c.unwrap_or(0) + r.unwrap_or(0)
		}));
		let depth = t.clone().fold_up(Rc::new(|l: Option<usize>,c: Option<usize>,r: Option<usize>| {
			match c {
				None => max(l.unwrap(),r.unwrap()) + 1,
				Some(_) => 1,
			}
		}));
		let in_order = t.clone().fold_up(Rc::new(|l: Option<bool>,c: Option<usize>,r: Option<bool>|{
			match c {
				None => l.unwrap() >= r.unwrap(),
				Some(_) => true,
			}
		}));
		assert_eq!(21, sum);
		assert_eq!(5, depth);
		assert_eq!(true, in_order);
	}
}


