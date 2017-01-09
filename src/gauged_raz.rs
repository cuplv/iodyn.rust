// Gauged RAZ - sequence
// - cursor access in low-const O(1) time
// - arbirary access in O(log n) time
// - combines tree cursor with stack

use std::rc::Rc;

use split_btree_cursor as tree;
use gauged_stack as stack;

#[derive(Clone)]
struct Raz<E: Clone> {
	l_forest: tree::Cursor<TreeData<E>>,
	l_stack: stack::GStack<E,Option<tree::Level>>,
	r_stack: stack::GStack<E,Option<tree::Level>>,
	r_forest: tree::Cursor<TreeData<E>>,
}

#[derive(Clone)]
enum TreeData<E> {
	Branch{l_count: usize, r_count: usize},
	Leaf(Rc<Vec<E>>),
} 
#[derive(Clone)]
struct RazTree<E: Clone>{count: usize, tree: tree::Tree<TreeData<E>>}

impl<E:Clone> RazTree<E> {
	pub fn focus(self, mut pos: usize) -> Option<Raz<E>> {
		match self { RazTree{count,tree} => {
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
			let mut l_gstack = stack::GStack::new(None);
			let mut r_gstack = stack::GStack::new(None);
			l_gstack.extend(l_slice);
			r_gstack.extend_rev(r_slice);
			// step 3: integrate
			Some(Raz{
				l_forest: l_cursor,
				l_stack: l_gstack,
				r_stack: r_gstack,
				r_forest: r_cursor,
			})
		}}
	}

}

impl<E:Clone> Raz<E> {
	pub fn new() -> Raz<E> {
		Raz{
			l_forest: tree::Cursor::new(),
			l_stack: stack::GStack::new(None),
			r_stack: stack::GStack::new(None),
			r_forest: tree::Cursor::new(),
		}
	}
	pub fn unfocus(mut self) -> RazTree<E> {
		// step 1: reconstruct local array from stack
		let mut l_vec = if self.l_stack.get_meta().is_none() {
			if let Some((_,vec)) = self.l_stack.pop_vec() {
				Some(vec)
			} else { None }
		} else { None };
		let mut r_vec = if self.r_stack.get_meta().is_none() {
			if let Some((_,vec)) = self.r_stack.pop_vec() {
				Some(vec)
			} else { None }
		} else { None };
		let vec = match (self.l_stack.is_empty(), l_vec, r_vec, self.r_stack.is_empty()) {
			(_,Some(v),None,_) => Some(v),
			(_,None,Some(mut v),_) => { v.reverse(); Some(v) },
			(_,Some(mut lv),Some(mut rv),_) => {rv.reverse(); lv.extend(rv); Some(lv) },
			(false, None, None, _) => {
				let (_,v) = self.l_stack.pop_vec().unwrap();
				Some(v)
			},
			(true, None, None, false) => {
				let (_,mut v) = self.r_stack.pop_vec().unwrap();
				v.reverse();
				Some(v)
			},
			_ => None
		};
		// step 2: build center tree
		let tree = if let Some(v) = vec {
			// OPTIMIZE: linear algorithm
			unimplemented!()
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
		let mut join_cursor = tree.into();
		if self.l_forest.up() {
			let h = self.l_forest.peek_level().unwrap();
			self.l_forest.down_left_discard();
			join_cursor = tree::Cursor::join(self.l_forest,h,TreeData::Branch{l_count: 0, r_count: 0},join_cursor);
		}
		if self.r_forest.up() {
			let h = self.r_forest.peek_level().unwrap();
			self.r_forest.down_right_discard();
			join_cursor = tree::Cursor::join(join_cursor,h,TreeData::Branch{l_count: 0, r_count: 0},self.r_forest);
		}
		// step 4: convert to final tree
		while join_cursor.up() {}
		RazTree{count: 0, tree: join_cursor.at_tree()}
	}
	pub fn push_left(&mut self, elm: E) {
		self.l_stack.push(elm);
	}
	pub fn push_right(&mut self, elm: E) {
		self.r_stack.push(elm);
	}
	pub fn pop_left(&mut self) -> Option<E> {
		if self.l_stack.len() == 0 {
			if !self.l_forest.up() { return None } else {
				self.l_stack.set_meta(self.l_forest.peek_level());
				self.l_forest.down_left_discard();
				while self.l_forest.down_right() {}
				match self.l_forest.peek() {
					Some(&TreeData::Leaf(ref data)) => self.l_stack.extend(&***data),
					_ => panic!("pop_left: no left tree leaf"),
				}
			}
		}
		self.l_stack.pop()
	}
	pub fn pop_right(&mut self) -> Option<E> {
		if self.r_stack.len() == 0 {
			if !self.r_forest.up() { return None } else {
				self.r_stack.set_meta(self.r_forest.peek_level());
				self.r_forest.down_right_discard();
				while self.r_forest.down_left() {}
				match self.r_forest.peek() {
					Some(&TreeData::Leaf(ref data)) => self.r_stack.extend_rev(&***data),
					_ => panic!("pop_right: no right tree leaf"),
				}
			}
		}
		self.r_stack.pop()
	}
}

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
  	let mut tree = RazTree{
  		size: 12,
  		tree: tree::Tree::new(5,TreeData::Branch{l_items:8},
  			tree::Tree::new(3,TreeData::Branch{l_items:2},
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(1,2))),tree::Tree::empty(),tree::Tree::empty()),
  				tree::Tree::new(2,TreeData::Branch{l_items:4},
  					tree::Tree::new(1,TreeData::Branch{l_items:2},
		  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(3,4))),tree::Tree::empty(),tree::Tree::empty()),
		  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(5,6))),tree::Tree::empty(),tree::Tree::empty()),
  					),
  					tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(7,8))),tree::Tree::empty(),tree::Tree::empty()),
  				)
  			),
  			tree::Tree::new(4,TreeData::Branch{l_items: 2},
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(9,10))),tree::Tree::empty(),tree::Tree::empty()),
  				tree::Tree::new(0,TreeData::Leaf(Rc::new(vec!(11,12))),tree::Tree::empty(),tree::Tree::empty()),
  			)
  		)
  	};
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
}