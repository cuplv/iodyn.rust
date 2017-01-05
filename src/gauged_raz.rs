// Gauged RAZ - sequence
// - cursor access in low-const O(1) time
// - arbirary access in O(log n) time
// - combines tree cursor with stack

use std::collections::VecDeque;
use std::rc::Rc;

use split_btree_cursor as tree;
use gauged_stack as stack;

struct Raz<E: Clone> {
	l_forest: tree::Cursor<TreeData<E>>,
	l_stack: stack::GStack<E,()>,
	l_zip: Vec<E>,
	r_zip: VecDeque<E>,
	r_stack: stack::GStack<E,()>,
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
			let mut cursor: tree::Cursor<TreeData<E>> = tree.into();
			while let TreeData::Branch{l_items} = *cursor.peek().unwrap() {
				if size <= l_items {
					assert!(cursor.down_left());
				} else {
					pos -= l_items;
					assert!(cursor.down_right());
				}
			}
			let (l_cursor, tree, r_cursor) = cursor.split();
			let (l_vec,r_vecdeq) = match *tree.peek().unwrap() {
				TreeData::Branch{..} => unreachable!(),
				TreeData::Leaf(ref vec_ref) => {
					let (lv, rv) = vec_ref.split_at(pos);
					(lv.into(), Into::<Vec<_>>::into(rv).into())
				}
			};
			Some(Raz{
				l_forest: l_cursor,
				l_stack: stack::GStack::new(()),
				l_zip: l_vec,
				r_zip: r_vecdeq,
				r_stack: stack::GStack::new(()),
				r_forest: r_cursor,
			})
		}}
	}

}

impl<E:Clone> Raz<E> {

}