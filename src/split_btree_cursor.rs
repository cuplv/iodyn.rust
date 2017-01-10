// Split Binary Tree Cursor
// - a cursor within a persistent, ordered, binary tree
// - optimised for splitting and combining trees at the cursor in a cannonical way
// - uses non-increasing levels for each subtree to maintain cannonical form
// - in the general case the most efficent levels will be drawn from 
//   a negative binomial distribution

// TODO: find a good way to abstract the stack implementation; macro?

use std::mem;
use std::rc::Rc;
use pat::AsPattern;

pub struct Tree<E>(TreeLink<E>);
pub type Level = usize;

impl<E> Tree<E> {
	pub fn empty() -> Self { Tree(None) }
	pub fn new(level: Level, element: E, left_branch: Tree<E>, right_branch: Tree<E>) -> Tree<E> {
		let Tree(l) = left_branch;
		let Tree(r) = right_branch;
		Tree(Some((level,Rc::new(TreeNode{data: element, l_branch: l, r_branch: r}))))
	}
	pub fn level(&self) -> Level {
		let Tree(ref t) = *self;
		match *t {
			None => 0,
			Some((lev,_)) => lev,
		}
	}
	pub fn peek(&self) -> Option<&E> {
		let Tree(ref t) = *self;
		link_peek(t)
	}

}
impl<E> Clone for Tree<E> {
	fn clone(&self) -> Self {
		let Tree(ref t) = *self;
		Tree(t.clone())
	}
}

// deconstruction patterns
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
fn link_peek<E>(link: &TreeLink<E>) -> Option<&E>{
	link.as_ref().map(|&(_,ref node)| &node.data)
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

pub trait TreeUpdate {
	fn update(l_branch: Option<&Self>, old_data: &Self, r_branch: Option<&Self>) -> Self;
}
pub trait DeriveTreeUpdate{}
impl<E: DeriveTreeUpdate + Clone> TreeUpdate for E {
	#[allow(unused_variables)]
	fn update(l_branch: Option<&Self>, old_data: &Self, r_branch: Option<&Self>) -> Self { old_data.clone() }
}

// cursor movement qualifier
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Force {
	No,
	Yes,
	Discard,
}

impl<E: TreeUpdate> From<Tree<E>> for Cursor<E> {
	fn from(tree: Tree<E>) -> Self {
		let Tree(tree) = tree;
		Cursor{
			dirty: false,
			l_forest: Vec::new(),
			tree: tree,
			r_forest: Vec::new(),
		}
	}
}

impl<E: TreeUpdate> Cursor<E> {
	pub fn new() -> Self {
		Cursor{
			dirty: false,
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
			Some((_,ref t)) => match **t { TreeNode{ ref l_branch, ref r_branch, ..} =>
				(l_branch.clone(), r_branch.clone())
			}
		};
		(
			Cursor{
				dirty: true,
				l_forest: self.l_forest,
				tree: l_tree,
				r_forest: Vec::new(),
			},
			Tree(self.tree),
			Cursor{
				dirty: true,
				l_forest: Vec::new(),
				tree: r_tree,
				r_forest: self.r_forest,
			},
		)
	}

	// makes a new cursor at the given data, between the trees of the other cursors
	pub fn join(mut l_cursor: Self, level: Level, data: E, mut r_cursor: Self) -> Self {
		// step 1: remove center forests
		while !l_cursor.r_forest.is_empty() { assert!(l_cursor.up()); }
		while !r_cursor.l_forest.is_empty() { assert!(r_cursor.up()); }
		// step 2: find insertion point
		while let Some(h) = l_cursor.peek_level() {
			if h < level { break; }
			else { assert!(l_cursor.down_right_force(Force::Yes)); }
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

	pub fn at_tree(&self) -> Tree<E> {
		Tree(self.tree.clone())
	}

	pub fn left_tree(&self) -> Option<Tree<E>> {
		self.tree.as_ref().map(|&(_, ref t)| Tree(t.l_branch.clone()))
	}

	pub fn right_tree(&self) -> Option<Tree<E>> {
		self.tree.as_ref().map(|&(_, ref t)| Tree(t.r_branch.clone()))
	}

	pub fn peek(&self) -> Option<&E> {
		link_peek(&self.tree)
	}

	pub fn peek_level(&self) -> Option<Level> {
		self.tree.as_ref().map(|&(lev, _)| lev)
	}

	// move the cursor into the left branch, returning true if successful
	// based on force:
	// No, don't move it there is no branch
	// Yes, move into an empty branch
	// Discard: discards the current node from this cursor as it moves, 
	//   effectively connecting the upper branch to the left side.
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
	pub fn down_left(&mut self) -> bool { self.down_left_force(Force::No) }

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
	pub fn down_right(&mut self) -> bool { self.down_right_force(Force::No) }


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
					match *t { TreeNode{ref data, ref l_branch, ref r_branch} => {
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
					match *t { TreeNode{ref data, ref l_branch, ref r_branch} => {
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
