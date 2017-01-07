// Split Binary Tree Cursor
// - a cursor within a persistent, ordered, binary tree
// - optimised for splitting and combining trees at the cursor in a cannonical way
// - uses non-increasing heights for each subtree to maintain cannonical form
// - in the general case the most efficent heights will be drawn from 
//   a negative binomial distribution
// - allows and assumes that structural changes will be made: copies data during movement

// TODO: dirty flags in cursor to avoid unnessesary structural changes
// TODO: find a good way to abstract the stack implementation; macro?

use std::rc::Rc;
use pat::AsPattern;

#[derive(Clone)]
pub struct Tree<E>(TreeLink<E>);
pub type Height = usize;

impl<E> Tree<E> {
	pub fn empty() -> Self { Tree(None) }
	pub fn new(height: Height, element: E, left_branch: Tree<E>, right_branch: Tree<E>) -> Tree<E> {
		let Tree(l) = left_branch;
		let Tree(r) = right_branch;
		Tree(Some(Rc::new(TreeNode{height: height, data: element, l_branch: l, r_branch: r})))
	}
	pub fn height(&self) -> Height {
		let Tree(ref t) = *self;
		match *t {
			None => 0,
			Some(ref t) => t.height,
		}
	}
	pub fn peek(&self) -> Option<&E> {
		let Tree(ref t) = *self;
		t.as_ref().map(|ref node| &node.data)
	}

}

// deconstruction patterns
pub enum T<E> {
	None,
	Take(Height, Tree<E>, E, Tree<E>),
	Shared(Tree<E>),
}

struct TreeNode<E>{
	height: Height,
	data: E,
	l_branch: TreeLink<E>,
	r_branch: TreeLink<E>
}
type TreeLink<E> = Option<Rc<TreeNode<E>>>;


impl<E> AsPattern<T<E>> for Tree<E> {
	fn pat(self) -> T<E> {
		let Tree(node) = self;
		match node {
			None => T::None,
			Some(n) =>  match Rc::try_unwrap(n) {
				Ok(TreeNode{height,data,l_branch,r_branch}) => {
					T::Take(height, Tree(l_branch), data, Tree(r_branch))
				}
				Err(n) => T::Shared(Tree(Some(n)))
			}
		}
	}
}

#[derive(Clone)]
pub struct Cursor<E: Clone> {
	l_forest: Vec<(TreeLink<E>,Height,E)>,
	tree: TreeLink<E>,
	r_forest: Vec<(Height,E,TreeLink<E>)>,
}

impl<E: Clone> From<Tree<E>> for Cursor<E> {
	fn from(tree: Tree<E>) -> Self {
		let Tree(tree) = tree;
		Cursor{
			l_forest: Vec::new(),
			tree: tree,
			r_forest: Vec::new(),
		}
	}
}

impl<E: Clone> Cursor<E> {
	pub fn new() -> Self {
		Cursor{
			l_forest: Vec::new(),
			tree: None,
			r_forest: Vec::new(),
		}
	}
	// returns the current tree plus two cursors containing
	//   every node to the left and right, respectively, of the tree's top node.
	// the returned cursors will be located at the two branches of the returned tree
	pub fn split(self) -> (Cursor<E>, Tree<E>, Cursor<E>) {
		let (l_tree,r_tree) = match self.tree {
			None => (None, None),
			Some(ref t) => match **t { TreeNode{ ref l_branch, ref r_branch, ..} =>
				(l_branch.clone(), r_branch.clone())
			}
		};
		(
			Cursor{
				l_forest: self.l_forest,
				tree: l_tree,
				r_forest: Vec::new(),
			},
			Tree(self.tree),
			Cursor{
				l_forest: Vec::new(),
				tree: r_tree,
				r_forest: self.r_forest,
			},
		)
	}

	// makes a new cursor at the given data, between the trees of the other cursors
	pub fn join(mut l_cursor: Self, height: Height, data: E, mut r_cursor: Self) -> Self {
		// step 1: remove center forests
		while !l_cursor.r_forest.is_empty() { assert!(l_cursor.up()); }
		while !r_cursor.l_forest.is_empty() { assert!(r_cursor.up()); }
		// step 2: find insertion point
		while let Some(h) = l_cursor.peek_height() {
			if h < height { break; }
			else { assert!(l_cursor.force_down_right()); }
		}
		while let Some(h) = r_cursor.peek_height() {
			if h <= height { break; }
			else { assert!(r_cursor.force_down_left()); }
		}
		// step 3: build center tree
		let tree = Some(Rc::new(TreeNode{
			height: height,
			data: data,
			l_branch: l_cursor.tree.clone(),
			r_branch: r_cursor.tree.clone(),
		}));
		// step4: join structures
		Cursor{
			l_forest: l_cursor.l_forest,
			tree: tree,
			r_forest: r_cursor.r_forest,
		}
	}

	pub fn at_tree(&self) -> Tree<E> {
		Tree(self.tree.clone())
	}

	pub fn left_tree(&self) -> Option<Tree<E>> {
		self.tree.as_ref().map(|ref t| Tree(t.l_branch.clone()))
	}

	pub fn right_tree(&self) -> Option<Tree<E>> {
		self.tree.as_ref().map(|ref t| Tree(t.r_branch.clone()))
	}

	pub fn peek(&self) -> Option<&E> {
		self.tree.as_ref().map(|ref tree| &tree.data)
	}

	pub fn peek_height(&self) -> Option<Height> {
		self.tree.as_ref().map(|ref tree| tree.height)
	}

	// move into the left branch, if it exists
	pub fn down_left(&mut self) -> bool {
		let (new_tree, old_branch) = match self.tree {
			None => return false,
			Some(ref t) => {
				if t.l_branch.is_none() { return false }
				(
					t.l_branch.clone(),
					(t.height, t.data.clone(), t.r_branch.clone()),
				)
			}
		};
		self.r_forest.push(old_branch);
		self.tree = new_tree;
		true
	}

	// will move into an empty branch
	// return false if self is on an empty branch
	pub fn force_down_left(&mut self) -> bool {
		let (new_tree, old_branch) = match self.tree {
			None => return false,
			Some(ref t) => {
				(
					t.l_branch.clone(),
					(t.height, t.data.clone(), t.r_branch.clone()),
				)
			}
		};
		self.r_forest.push(old_branch);
		self.tree = new_tree;
		true
	}

	// discards the current node from this cursor as it moves, 
	// effectively connecting the upper branch to the left side.
	// does not fail if moving to an empty branch
	pub fn down_left_discard(&mut self) -> bool {
		let new_tree = match self.tree {
			None => return false,
			Some(ref t) => { t.l_branch.clone() }
		};
		self.tree = new_tree;
		true
	}

	pub fn down_right(&mut self) -> bool {
		let (new_tree, old_branch) = match self.tree {
			None => return false,
			Some(ref t) => {
				if t.r_branch.is_none() { return false }
				(
					t.r_branch.clone(),
					(t.l_branch.clone(), t.height, t.data.clone()),
				)
			}
		};
		self.l_forest.push(old_branch);
		self.tree = new_tree;
		true
	}

	pub fn force_down_right(&mut self) -> bool {
		let (new_tree, old_branch) = match self.tree {
			None => return false,
			Some(ref t) => {
				(
					t.r_branch.clone(),
					(t.l_branch.clone(), t.height, t.data.clone()),
				)
			}
		};
		self.l_forest.push(old_branch);
		self.tree = new_tree;
		true
	}

	// discards the current node from this cursor as it moves, 
	// effectively connecting the upper branch to the right side.
	// does not fail if moving to an empty branch
	pub fn down_right_discard(&mut self) -> bool {
		let new_tree = match self.tree {
			None => return false,
			Some(ref t) => { t.r_branch.clone() }
		};
		self.tree = new_tree;
		true
	}

	pub fn up(&mut self) -> bool {
		let use_left = match (self.l_forest.last(), self.r_forest.last()) {
			(None, None) => { return false },
			(Some(_), None) => true,
			(Some(&(_,l_height,_)), Some(&(r_height,_,_))) if r_height > l_height => true,
			_ => false,
		};
		if use_left {
			let (l_branch, l_height, l_data) = self.l_forest.pop().unwrap();
			self.tree = Some(Rc::new(TreeNode{
				height: l_height,
				data: l_data,
				l_branch: l_branch,
				r_branch: self.tree.take(),
			}));
		} else {
			let (r_height, r_data, r_branch) = self.r_forest.pop().unwrap();
			self.tree = Some(Rc::new(TreeNode{
				height: r_height,
				data: r_data,
				l_branch: self.tree.take(),
				r_branch: r_branch
			}));
		}
		return true;
	}

}
