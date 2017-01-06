// Gauged RAZ - sequence
// - cursor access in low-const O(1) time
// - arbirary access in O(log n) time
// - combines tree cursor with stack

use std::rc::Rc;

use split_btree_cursor as tree;
use gauged_stack as stack;

struct Raz<E: Clone> {
	l_forest: tree::Cursor<TreeData<E>>,
	l_stack: stack::GStack<E,Option<tree::Height>>,
	r_stack: stack::GStack<E,Option<tree::Height>>,
	r_forest: tree::Cursor<TreeData<E>>,
}

#[derive(Clone)]
enum TreeData<E> {
	Branch{l_items: usize},
	Leaf(Rc<Vec<E>>),
} 
struct RazTree<E: Clone>{size: usize, tree: tree::Tree<TreeData<E>>}

impl<E:Clone> RazTree<E> {
	pub fn focus(self, mut pos: usize) -> Option<Raz<E>> {
		match self { RazTree{size,tree} => {
			if size < pos { return None };
			let mut cursor = tree::Cursor::from(tree);
			while let TreeData::Branch{l_items} = *cursor.peek().unwrap() {
				if size <= l_items {
					assert!(cursor.down_left());
				} else {
					pos -= l_items;
					assert!(cursor.down_right());
				}
			}
			let (l_cursor, tree, r_cursor) = cursor.split();
			let (l_slice,r_slice) = match *tree.peek().unwrap() {
				TreeData::Branch{..} => unreachable!(),
				TreeData::Leaf(ref vec_ref) => vec_ref.split_at(pos)
			};
			let mut l_gstack = stack::GStack::new(None);
			let mut r_gstack = stack::GStack::new(None);
			l_gstack.extend(l_slice);
			r_gstack.extend_rev(r_slice);
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
	pub fn push_left(&mut self, elm: E) {
		self.l_stack.push(elm);
	}
	pub fn push_right(&mut self, elm: E) {
		self.r_stack.push(elm);
	}
	pub fn pop_left(&mut self) -> Option<E> {
		if self.l_stack.len() == 0 {
			if !self.l_forest.up() { return None } else {
				self.l_stack.set_meta(self.l_forest.peek_height());
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
				self.r_stack.set_meta(self.r_forest.peek_height());
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
}