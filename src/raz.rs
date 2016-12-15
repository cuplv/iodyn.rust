// Random access zipper

use std::rc::Rc;

use rand;

use stack::Stack;
use seqzip::{Seq, SeqZip};
use zip::Zip;

type Level = usize;
type Count = usize;

#[derive(Debug)]
pub enum Tree<T> {
    Leaf(Rc<T>),
    Branch(Level,Count,TreeLink<T>,TreeLink<T>)
}

type TreeLink<T> = Option<Rc<Tree<T>>>;

#[derive(Debug)]
struct RazSide<T> {
  forest: Stack<Tree<T>>,
  tree: TreeLink<T>,
  leaves: Stack<(Rc<T>,Level)>,
  left_trees: bool,
}

#[derive(Debug)]
pub struct Raz<T> {
	// publicly, side one is left, side two is right
	// internally they may have other uses, check the fn comments
	one: RazSide<T>,
	two: RazSide<T>,
  level: Level,
  count: Count,
}

fn gen_level() -> Level {
	// TODO: better generator, this is suitable for sequences < 50 items
	let options = vec![0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0];
	let index = rand::random::<usize>() % options.len();
	options[index]
}

fn count_tl<T>(tl: &TreeLink<T>) -> usize{
	match *tl { None => 0, Some(ref t) => t.count() }
}

impl<T> Tree<T> {
	fn count(&self) -> usize {
		match *self {
			Tree::Leaf(_) => 1,
			Tree::Branch(_,size,_,_) => size,
		}
	}
}

impl<T> Raz<T> {
	fn new() -> Raz<T> {
		Raz{
			one: RazSide::new(true),
			two: RazSide::new(false),
			level: gen_level(),
			count: 0
		}
	}

	fn empty(level: Level) -> Raz<T> {
		Raz{
			one: RazSide::new(true),
			two: RazSide::new(false),
			level: level,
			count: 0
		}
	}

}

impl<T> Clone for Raz<T> {
  fn clone(&self) -> Self {
  	Raz{
  		one: self.one.clone(),
  		two: self.two.clone(),
  		level: self.level,
  		count: self.count
  	}
  }
}

impl<T> Clone for RazSide<T> {
  fn clone(&self) -> Self {
    RazSide {
    	forest: self.forest.clone(),
    	tree: self.tree.clone(),
    	leaves: self.leaves.clone(),
    	left_trees: self.left_trees,
    }
  }
}

impl<T> RazSide<T> {
	fn new(left_trees: bool) -> RazSide<T> { RazSide { forest: Stack::new(), tree: None, leaves: Stack::new(), left_trees: left_trees }}

	// prepares for access, returning a side with leaves, or one completely empty
	// returns None if no trim was necessary
	fn trim(&self) -> Option<Self> {
		if !self.needs_trim() { None } else { Some({
		let mut forest = self.forest.clone();
		let mut tree = self.tree.clone();
		while let Tree::Branch(lev,_,ref t1,ref t2) = *tree.take().unwrap_or(panic!("misplaced empty tree")) {
			if self.left_trees {
			  forest = forest.push(Rc::new(Tree::Branch(lev, count_tl(t1), t1.clone(), None)));
			  tree = t2.clone();
			} else {
			  forest = forest.push(Rc::new(Tree::Branch(lev, count_tl(t2), None, t2.clone())));
			  tree = t1.clone();
			}
		}
		let next_elem = match *tree.unwrap_or(panic!("misplaced empty tree")) {
			Tree::Branch(_,_,_,_) => panic!("bad while above"),
			Tree::Leaf(ref dat) => dat.clone(),
		};
		let next_branch = forest.peek().unwrap_or(panic!("has final element"));
		let (next_level, next_tree) = match *next_branch {
			Tree::Leaf(_) => panic!("misshapen forest"),
			Tree::Branch(lev,_,ref t1,ref t2) => if self.left_trees {(lev,t1.clone())} else {(lev,t2.clone())}
		};
		RazSide{
			forest: forest.pull().unwrap_or(panic!("this panic happened above at peek")),
			tree: next_tree,
			leaves: Stack::new().push(Rc::new((next_elem, next_level))),
			.. *self
		}})}
	}

	// sides need a trim if they are not empty and there are no leaves available
	fn needs_trim(&self) -> bool {
		self.leaves.is_empty() & self.tree.is_some()
	}

	fn push(&self, val: Rc<T>, level: Level) -> Self {
		RazSide{
			forest: self.forest.clone(), 
			tree: self.tree.clone(),
			leaves: self.leaves.push(Rc::new((val,level))),
			.. *self
		}
	}

	// pulls from this side, returning the 'tail'
	fn pull(&self) -> Option<RazSide<T>> {
		let trimmed; // anchor binding for trimmed data if necessary
		let side = match self.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => self };
		side.leaves.pull().map(|leaves| RazSide {
			forest: self.forest.clone(),
			tree: self.tree.clone(),
			leaves: leaves,
			.. *self
		})
	}
}

impl<T: Clone> Raz<T> {
	// pushes to side one
	fn push_to(raz: &Raz<T>, val: Rc<T>, level: Level) -> Raz<T> {
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
				two: RazSide { leaves: from.leaves.push(Rc::new((elm,raz.level))), forest: from.forest.clone(), tree: from.tree.clone(), .. *from},
				level: lev,
				count: raz.count
			}
		})
	}

}

impl<T: Clone> Zip<T> for Raz<T> {

	// peek throws away the work of trimming!
	// TODO: rewrite to work without trimming
	fn peek_l(&self) -> Result<Rc<T>,&str> {
		let trimmed; // anchor binding for trimmed data if necessary
		let trim = match self.one.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => &self.one };
		match trim.leaves.peek() {
			Some(dat) => { let (ref elm, _) = *dat; Ok(elm.clone()) },
			None => Err("Raz: Peek past beginning of sequence")
		}
	}
	fn peek_r(&self) -> Result<Rc<T>,&str> {
		let trimmed; // anchor binding for trimmed data if necessary
		let trim = match self.two.trim() { Some(trim) => {trimmed = trim; &trimmed}, None => &self.two };
		match trim.leaves.peek() {
			Some(dat) => { let (ref elm, _) = *dat; Ok(elm.clone()) },
			None => Err("Raz: Peek past end of sequence")
		}
	}
	fn push_l(&self, val: Rc<T>) -> Self {
		Self::push_to(&self, val, gen_level())
	}
	fn push_r(&self, val: Rc<T>) -> Self {
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
		match self.one.pull() {
			Some(two) => Ok(Raz{ one: self.one.clone() , two: two, level: self.level, count: self.count - 1 }),
			None => Err("Raz: Pull past end of sequence")
		}
	}
	// zip implemented to maintain levels while moving
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

impl<T: Clone> Seq<T,Raz<T>> for Tree<T> {
	fn zip_to(&self, loc: usize) -> Result<Raz<T>,&str> {
		unimplemented!()
	}
}

impl<T: Clone> SeqZip<T,Tree<T>> for Raz<T> {
	fn unzip(&self) -> Tree<T> {
		unimplemented!()
	}
}
