//! Example data to use in tests
//!
//! Legend:
//! - Name: {a}
//! - Level: (1)
//! - Data as arrays: [1,2,3]

use iodyn::{IRaz, IRazTree};
use interface::Adapt;
use adapton::engine::*;

/// An example binary tree shaped like this:
///
/// ```text
///                 __________{t}(6)___________
///                /                           \
///         ___{l}(4)__                  _____{r}(5)____
///        /           \                /               \
///     {ll}(1)       {lr}(2)       {rl}(3)           {rr}(4)
///    /     \	     /     \       /      \          /      \
/// [1,2]  [3,4]  [5,6]  [7,8]  [9,10]  [11,12]  [13,14]  [15,16]
/// ```
///
pub fn iraztree_depth_4<A:Adapt+From<usize>>() -> IRazTree<A> {
  	let mut r = IRaz::new();
  	r.push_left(A::from(1));
  	r.push_left(A::from(2));
  	r.archive_left(1,Some(name_of_string(String::from("ll"))));
  	r.push_left(A::from(3));
  	r.push_left(A::from(4));
  	r.archive_left(4,Some(name_of_string(String::from("l"))));
  	r.push_left(A::from(5));
  	r.push_left(A::from(6));
  	r.archive_left(2,Some(name_of_string(String::from("lr"))));
  	r.push_left(A::from(7));
  	r.push_left(A::from(8));
  	r.archive_left(6,Some(name_of_string(String::from("rl"))));
  	r.push_left(A::from(9));
  	r.push_left(A::from(10));
  	r.archive_left(3,Some(name_of_string(String::from("r"))));
  	r.push_left(A::from(11));
  	r.push_left(A::from(12));
  	r.archive_left(5,Some(name_of_string(String::from("rr"))));
  	r.push_left(A::from(13));
  	r.push_left(A::from(14));
  	r.archive_left(4,Some(name_of_string(String::from("g"))));
  	r.push_left(A::from(15));
  	r.push_left(A::from(16));
  	r.unfocus()
}