//! Random access zipper, early version
//!
//! This version uses a tree structure to
//! represent a sequence. The tree contains
//! one element per leaf, so the search depth
//! may be higher than the version in `gauged_raz`.
//! This version is fully persistent, with
//! every edit sharing all other data. Cloning
//! is fast and expected

use std::intrinsics;
use std::rc::Rc;
//use std::fmt::Debug;

use rand;

use stack::Stack;
use seqzip::{Seq, SeqZip};
use zip::Zip;

type Level = usize;
type Count = usize;

#[derive(Clone)]
pub struct RazSeq<T>(TreeLink<T>);

pub struct Raz<T> {
	// publicly, side one is left, side two is right
	// internally they may have other uses, check the fn comments
	one: RazSide<T>,
	two: RazSide<T>,
  level: Level,
  count: Count,
}

#[derive(Clone)]
enum Tree<T> {
    Leaf(T),
    Branch(Level,Count,TreeLink<T>,TreeLink<T>)
}

type TreeLink<T> = Option<Rc<Tree<T>>>;

struct RazSide<T> {
  forest: Stack<Tree<T>>,
  tree: TreeLink<T>,
  leaves: Stack<(T,Level)>,
  left_trees: bool,
}

pub struct Iter<T> {
  next: Raz<T>,
}

fn gen_level() -> Level {
	// // TODO: better generator, this is suitable for sequences < 50 items
	// let options = vec![0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0];
	// let index = rand::random::<usize>() % options.len();
	// options[index] + 1 // level 0 is for empty trees and leaves

	let num = rand::random::<usize>();
	let bits = unsafe{ intrinsics::ctlz(num)};
	bits+1
}

fn count_tl<T>(tl: &TreeLink<T>) -> usize {
	match *tl { None => 0, Some(ref t) => t.count() }
}

fn level_tl<T>(tl: &TreeLink<T>) -> usize {
	match *tl { None => 0, Some(ref t) => t.level() }
}

fn level_to<T>(tl: &Option<&Tree<T>>) -> usize {
	match *tl { None => 0, Some(t) => t.level() }
}

impl<T> Tree<T> {
	fn count(&self) -> usize {
		match *self {
			Tree::Leaf(_) => 1,
			Tree::Branch(_,size,_,_) => size,
		}
	}
	fn level(&self) -> usize {
		match *self {
			Tree::Leaf(_) => 0,
			Tree::Branch(lev,_,_,_) => lev
		}
	}
}

impl<T: Clone> RazSeq<T> {
  pub fn iter(&self) -> Iter<T> {
  	Iter{ next: self.zip_to(0).unwrap() }
  }
}

impl<T: Clone> Raz<T> {
	pub fn new() -> Raz<T> {
		Raz{
			one: RazSide::new(true),
			two: RazSide::new(false),
			level: gen_level(),
			count: 0
		}
	}

	pub fn empty(level: Level) -> Raz<T> {
		Raz{
			one: RazSide::new(true),
			two: RazSide::new(false),
			level: level,
			count: 0
		}
	}

	pub fn iter_r(&self) -> Iter<T> {
		Iter{ next: self.clone() }
	}

}

impl<T: Clone> Iterator for Iter<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
  	self.next.peek_r().ok().map(|val|{
	  	self.next = self.next.pull_r().unwrap();
	  	val	
  	})
  }
}

impl<T: Clone> Clone for Raz<T> {
  fn clone(&self) -> Self {
  	Raz{
  		one: self.one.clone(),
  		two: self.two.clone(),
  		level: self.level,
  		count: self.count
  	}
  }
}

impl<T: Clone> Clone for RazSide<T> {
  fn clone(&self) -> Self {
    RazSide {
    	forest: self.forest.clone(),
    	tree: self.tree.clone(),
    	leaves: self.leaves.clone(),
    	left_trees: self.left_trees,
    }
  }
}

impl<T: Clone> RazSide<T> {
	fn new(left_trees: bool) -> RazSide<T> { RazSide { forest: Stack::new(), tree: None, leaves: Stack::new(), left_trees: left_trees }}

	// prepares for access, returning a side with leaves
	// returns None if no trim was necessary, including a completely empty side
	fn trim(&self) -> Option<Self> {
		if !self.needs_trim() { None } else { Some({
		let mut forest = self.forest.clone();
		let mut tree = self.tree.clone();
		while let Tree::Branch(lev,_,ref t1,ref t2) = *match tree.clone() { None => panic!("misplaced empty tree"), Some(t) => t} {
			if self.left_trees {
			  forest = forest.push(Tree::Branch(lev, count_tl(t1), t1.clone(), None));
			  tree = t2.clone();
			} else {
			  forest = forest.push(Tree::Branch(lev, count_tl(t2), None, t2.clone()));
			  tree = t1.clone();
			}
		}
		let next_elem = match *match tree { None => panic!("misplaced empty tree"), Some(e) => e } {
			Tree::Branch(_,_,_,_) => panic!("bad while above"),
			Tree::Leaf(ref dat) => dat.clone(),
		};
		let next_branch = match forest.peek() {None => panic!("has final element"), Some(f) => f};
		let (next_level, next_tree) = match *next_branch {
			Tree::Leaf(_) => panic!("misshapen forest"),
			Tree::Branch(lev,_,ref t1,ref t2) => if self.left_trees {(lev,t1.clone())} else {(lev,t2.clone())}
		};
		RazSide{
			forest: match forest.pull() {None => panic!("this panic happened above at peek"), Some(f) => f},
			tree: next_tree,
			leaves: Stack::new().push((next_elem, next_level)),
			.. *self
		}})}
	}

	// sides need a trim if they are not empty and there are no leaves available
	fn needs_trim(&self) -> bool {
		self.leaves.is_empty() & self.tree.is_some()
	}

	fn push(&self, val: T, level: Level) -> Self {
		RazSide{
			forest: self.forest.clone(), 
			tree: self.tree.clone(),
			leaves: self.leaves.push((val,level)),
			.. *self
		}
	}

	// pulls from this side, returning the 'tail'
	fn pull(&self) -> Option<RazSide<T>> {
		let trimmed; // anchor binding for trimmed data if necessary
		let trim = match self.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => self };

		trim.leaves.pull().map(|leaves| {
			RazSide {
				forest: trim.forest.clone(),
				tree: trim.tree.clone(),
				leaves: leaves,
				.. *trim
			}
		})
	}
}

impl<T: Clone> Raz<T> {
	// pushes to side one
	fn push_to(raz: &Raz<T>, val: T, level: Level) -> Raz<T> {
		Raz{
			one: raz.one.push(val,level),
			two: raz.two.clone(),
			level: raz.level,
			count: raz.count + 1
		}
	} 

	// uses side one as the `to` side, side two as `from`
	fn zip_tf(raz: &Raz<T>) -> Option<Raz<T>> {
		let trimmed; // anchor binding for trimmed data if necessary
		let to = match raz.one.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => &raz.one };
		let from = &raz.two;
		to.leaves.peek().map(|dat| {
			let (ref elm, lev) = *dat;
			let elm = elm.clone();
			Raz{
				one: RazSide { leaves: to.leaves.pull().unwrap(), forest: to.forest.clone(), tree: to.tree.clone(), .. *to},
				two: RazSide { leaves: from.leaves.push((elm,raz.level)), forest: from.forest.clone(), tree: from.tree.clone(), .. *from},
				level: lev,
				count: raz.count
			}
		})
	}

}

impl<T: Clone> Zip<T> for Raz<T> {

	// peek throws away the work of trimming!
	// TODO: rewrite to work without trimming
	fn peek_l(&self) -> Result<T,&str> {
		let trimmed; // anchor binding for trimmed data if necessary
		let trim = match self.one.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => &self.one };
		match trim.leaves.peek() {
			Some(dat) => { let (ref elm, _) = *dat; Ok(elm.clone()) },
			None => Err("Raz: Peek past beginning of sequence")
		}
	}
	fn peek_r(&self) -> Result<T,&str> {
		let trimmed; // anchor binding for trimmed data if necessary
		let trim = match self.two.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => &self.two };
		match trim.leaves.peek() {
			Some(dat) => { let (ref elm, _) = *dat; Ok(elm.clone()) },
			None => Err("Raz: Peek past end of sequence")
		}
	}
	fn push_l(&self, val: T) -> Self {
		Self::push_to(&self, val, gen_level())
	}
	fn push_r(&self, val: T) -> Self {
		let rev = Self::push_to(
			&Raz{one: self.two.clone(), two: self.one.clone(), .. *self},
			val,gen_level()
		);
		Raz{one: rev.two.clone(), two: rev.one.clone(), .. rev }
	}
	fn pull_l(&self) -> Result<Self,&str> {
		match self.one.pull() {
			Some(one) => Ok(Raz{ one: one, two: self.two.clone(), level: self.level, count: self.count - 1 }),
			None => Err("Raz: Pull past beginning of sequence")
		}
	}
	fn pull_r(&self) -> Result<Self,&str> {
		match self.two.pull() {
			Some(two) => Ok(Raz{ one: self.one.clone() , two: two, level: self.level, count: self.count - 1 }),
			None => Err("Raz: Pull past end of sequence")
		}
	}
	// zip implemented to maintain levels while moving
	// TODO: implement other methods to save work and maintain levels
	fn zip_l(&self) -> Result<Self,&str> {
		match Self::zip_tf(&self) {
			Some(raz) => Ok(raz),
			None => Err("Raz: Move past beginning of sequence")
		}
	}
	fn zip_r(&self) -> Result<Self,&str> {
		match Self::zip_tf(&Raz{one: self.two.clone(), two: self.one.clone(), .. *self}) {
			Some(raz) => Ok(Raz{one: raz.two, two: raz.one, .. raz}),
			None => Err("Raz: Move past end of sequence")
		}
	}
}

impl<T: Clone> Seq<T,Raz<T>> for RazSeq<T> {
	fn zip_to(&self, loc: usize) -> Result<Raz<T>,&str> {
		let RazSeq(mut tree) = self.clone();
		let max_size = count_tl(&tree);
		if loc > max_size { return Err("Raz: zip_to past end of sequence")};
		let mut forest1 = Stack::new();
		let mut forest2 = Stack::new();
		let mut loc = loc;
		while let Tree::Branch(level,_,ref t1, ref t2) = *tree.take().expect("unexpected empty branch") {
	    let left_count = count_tl(t1);
	    if loc == left_count {
				return Ok(Raz{
					one: RazSide {
						forest: forest1,
						tree: t1.clone(),
						leaves: Stack::new(),
						left_trees: true,
					},
					two: RazSide {
						forest: forest2,
						tree: t2.clone(),
						leaves: Stack::new(),
						left_trees: false,
					},
					level: level,
					count: max_size,
				})
	    } else if loc < left_count {
				forest2 = forest2.push(
					Tree::Branch(level, count_tl(&t2), None, t2.clone())
				);
				tree = t1.clone();
	    } else {
				forest1 = forest1.push(
					Tree::Branch(level, count_tl(&t1), t1.clone(), None)
				);
				tree = t2.clone();
				loc = loc - left_count;
	    }
		}
		debug_assert!(true,"incomplete tree");
    Err("incomplete tree")
	}
}

// TODO: handle far left/right None trees?
fn integrate_forests<T: Clone>(
	mut l_forest: Stack<Tree<T>>,
	mut l_tree: TreeLink<T>,
	level: Level,
	mut r_tree: TreeLink<T>,
	mut r_forest: Stack<Tree<T>>,
	leave_left: bool,
	leave_right: bool
) -> (Stack<Tree<T>>,TreeLink<T>,Stack<Tree<T>>) {

	// step one: shift forests until level is between tree and forest
	// raise left side
	while level_tl(&l_tree) < level {
		if let Some(higher) = l_forest.peek().take() {
			if let Tree::Branch(l,c,ref t1,None) = *higher {
				l_tree = Some(Rc::new(
					Tree::Branch(l,c+count_tl(&l_tree),t1.clone(),l_tree.clone())
				));
			} else { debug_assert!(true, "integrate_forests: poorly constructed forest");}
		} else { break; }
		l_forest = l_forest.pull().unwrap();
	}
	// lower left side
	// OPTIMISE: avoid the final pull/push by checking level inside the forest
	while level_tl(&l_tree) >= level {
		match *match l_tree { None => panic!("level was 0"), Some(t) => t} {
			Tree::Leaf(_) => panic!("level was 0"),
			Tree::Branch(l,_,ref t1,ref t2) => {
				l_forest = l_forest.push(
					Tree::Branch(l,count_tl(&t1),t1.clone(),None )
				);
				l_tree = t2.clone();
			}
		}
	}
	// raise right side
	while level_tl(&r_tree) <= level {
		if let Some(higher) = r_forest.peek().take() {
			if let Tree::Branch(l,c,None,ref t2) = *higher {
				r_tree = Some(Rc::new(
					Tree::Branch(l,c+count_tl(&r_tree),r_tree.clone(),t2.clone())
				));
			} else { debug_assert!(true, "integrate_forests: poorly constructed forest")}
		} else { break; }
		r_forest = r_forest.pull().unwrap();
	}
	// lower right side
	// OPTIMISE: avoid the final pull/push by checking level inside the forest
	while level_tl(&r_tree) > level {
		match *match r_tree { None => panic!("level was 0"), Some(t) => t } {
			Tree::Leaf(_) => panic!("level was 0"),
			Tree::Branch(l,_,ref t1,ref t2) => {
				r_forest = r_forest.push(
					Tree::Branch(l,count_tl(&t2),None,t2.clone())
				);
				r_tree = t1.clone();
			}
		}
	}
	// step two: make center tree
	let mut center_tree = Some(Rc::new(Tree::Branch(
		level,
		count_tl(&l_tree)+count_tl(&r_tree),
		l_tree.clone(),
		r_tree.clone()
	)));
	// step three: build from forests to tree, stopping when indicated				
	if (leave_left | l_forest.is_empty()) & (leave_right | r_forest.is_empty()) {
		return (l_forest,center_tree,r_forest)
	}
	while !l_forest.is_empty() | !r_forest.is_empty() {
		let next_l_level = level_to(&l_forest.peek());
		let next_r_level = level_to(&r_forest.peek());
		if ((next_l_level < next_r_level) & !l_forest.is_empty()) | r_forest.is_empty() {
      if let Tree::Branch(l,c,ref t1,None) = *l_forest.peek().expect("checked empty above") {
        center_tree = Some(Rc::new(
        	Tree::Branch(l,c+count_tl(&center_tree),t1.clone(),center_tree.clone())
        ));
      } else { panic!("poorly constructed l_forest"); }
      l_forest = l_forest.pull().unwrap();
      if l_forest.is_empty() & leave_right { return (l_forest,center_tree,r_forest) };
    } else {
      if let Tree::Branch(l,c,None,ref t2) = *r_forest.peek().expect("both forests empty") {
        center_tree = Some(Rc::new(
        	Tree::Branch(l,c+count_tl(&center_tree),center_tree.clone(),t2.clone())
        ));
      } else { panic!("poorly constructed r_forest"); }
      r_forest = r_forest.pull().unwrap();
      if r_forest.is_empty() & leave_left { return (l_forest,center_tree,r_forest) };
    }
	}
	(l_forest,center_tree,r_forest)
}
// TODO:: implement these build_'s as O(n) trampolines
fn build_tree_left<T: Clone>(elms: &Stack<(T,Level)>) -> Option<(Level,TreeLink<T>, Stack<Tree<T>>)> {
	let (ref elm, mut level) = *match elms.peek() { None => return None, Some(e) => e };
	let mut tree = Some(Rc::new(Tree::Leaf(elm.clone())));
	let mut r_forest = Stack::new();
	let mut l_stack = elms.pull().unwrap();
	while !l_stack.is_empty() {
		if let Some(&(ref elm,lev)) = l_stack.peek() {
			let elm_as_tree = Some(Rc::new(Tree::Leaf(elm.clone())));
			let (_,t,f) = integrate_forests(Stack::new(), elm_as_tree, level, tree, r_forest, false, true);
			tree = t;
			r_forest = f;
			level = lev;
		}
		l_stack = l_stack.pull().unwrap();
	}
	Some((level,tree,r_forest))
}
fn build_tree_right<T: Clone>(elms: &Stack<(T,Level)>) -> Option<(Stack<Tree<T>>,TreeLink<T>, Level)> {
	let (ref elm, mut level) = *match elms.peek() { None => return None, Some(e) => e };
	let mut tree = Some(Rc::new(Tree::Leaf(elm.clone())));
	let mut l_forest = Stack::new();
	let mut r_stack = elms.pull().unwrap();
	while !r_stack.is_empty() {
		if let Some(&(ref elm,lev)) = r_stack.peek(){
			let elm_as_tree = Some(Rc::new(Tree::Leaf(elm.clone())));
			let (f,t,_) = integrate_forests(l_forest, tree, level, elm_as_tree, Stack::new(), true, false);
			tree = t;
			l_forest = f;
			level = lev;
		}
		r_stack = r_stack.pull().unwrap();
	}
	Some((l_forest,tree,level))
}

impl<T: Clone> SeqZip<T,RazSeq<T>> for Raz<T> {
	fn unzip(&self) -> RazSeq<T> {
		let (lf,lt,_) = { match build_tree_left(&self.one.leaves) {
			None => (self.one.forest.clone(), self.one.tree.clone(), Stack::new()),
			Some((lev,t,f)) => integrate_forests(self.one.forest.clone(), self.one.tree.clone(), lev, t, f, true, false),
		}};
		let (_,rt,rf) = { match build_tree_right(&self.two.leaves) {
			None => (self.one.forest.clone(), self.one.tree.clone(), Stack::new()),
			Some((f,t,lev)) => integrate_forests(f, t, lev, self.two.tree.clone(),self.two.forest.clone(), false, true),
		}};
		let (_,main_tree,_) = integrate_forests(lf,lt,self.level,rt,rf, false,false);
		RazSeq(main_tree)
	}
}

// avoid recursive drop on branches (but takes longer)
// impl<T> Drop for Tree<T> {
// 	fn drop(&mut self) {
// 		let mut pending_drop = Vec::new();
// 		match *self {
// 			Tree::Leaf(..) => {},
// 			Tree::Branch(_,_,ref mut t1,ref mut t2) => {
// 				if let Some(tree) = t1.take() {pending_drop.push(tree);}
// 				if let Some(tree) = t2.take() {pending_drop.push(tree);}
// 			}
// 		}
// 		while let Some(shared_tree) = pending_drop.pop() {
// 		  if let Ok(mut my_tree) = Rc::try_unwrap(shared_tree) {
// 		   	match my_tree {
// 					Tree::Leaf(..) => {},
// 					Tree::Branch(_,_,ref mut t1,ref mut t2) => {
// 						if let Some(tree) = t1.take() {pending_drop.push(tree);}
// 						if let Some(tree) = t2.take() {pending_drop.push(tree);}
// 					}
// 		   	}
// 		  }
// 		}
// 	}
// }
