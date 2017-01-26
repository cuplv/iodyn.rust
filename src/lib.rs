#![feature(core_intrinsics)]

extern crate rand;
extern crate pat;
//#[macro_use]
extern crate adapton;

pub mod zip;                // trait for persistent zips
pub mod stack;              // persistent stack
pub mod seqzip;             // traits for persistent raz
pub mod persist_raz;        // monolithic single-item persistent raz
pub mod trees;              // traits for the various forms of trees
pub mod level_tree;         // persistent tree
pub mod tree_cursor;        // splittable cursor over tree (uses level_tree)
pub mod archive_stack;      // more complex stack (uses stack)
pub mod gauged_raz;         // raz of vectors using tree_cursor (uses archive_stack and tree_cursor)
// temp for incremental use
pub mod inc_level_tree;
pub mod inc_tree_cursor;
pub mod inc_gauged_raz;

// tests on early mods only (persist_raz)
#[cfg(test)]
mod tests {
	use super::*;
	use seqzip::{AtLeft,AtRight};
	use zip::Zip;
	use seqzip::{Seq, SeqZip};
  use persist_raz::Raz;

  #[test]
  fn test_stack_zipper() {
  	// define a sequence
  	let none = stack::Stack::new();
  	let some = none.push(3).push(7).push(1).push(0);
  	{
		 	let result = some.iter().collect::<Vec<_>>();
		  assert_eq!(vec!(&0,&1,&7,&3), result);
		}

  	// save some of it for later
  	let save = some.pull().unwrap().pull().unwrap();

  	// use a zip to edit it
  	let cur = AtLeft(some).zip_to(2).unwrap();
  	assert_eq!(Ok(1), cur.peek_l());
  	let fix = cur.edit(zip::Dir::R, 2).unwrap();

  	// upzip back to a sequence to see the result
  	let restore = SeqZip::<_,AtLeft<_>>::unzip(&fix);
  	let result = restore.iter().collect::<Vec<_>>();
  	assert_eq!(vec!(&0,&1,&2,&3), result);

  	// unzip the other way the other way
  	let back: AtRight<_> = fix.unzip();
  	let result: Vec<_> = back.iter().collect();
  	assert_eq!(vec!(&3,&2,&1,&0), result);

  	// show off that this is a persistent structure
  	assert_eq!(Some(&7), save.peek());
  }

  #[test]
  fn test_raz_zipper() {
  	// define a sequence
  	let none = Raz::new();
  	let some = none.push_r(3).push_r(7).push_r(1).push_r(0);
	 	let result = some.iter_r().collect::<Vec<_>>();
	  assert_eq!(vec!(0,1,7,3), result);

  	// save some of it for later
  	let save = some.pull_r().unwrap().pull(zip::Dir::R).unwrap();

  	// use a zip to edit it
  	let _cur = some.unzip();
  	let cur = _cur.zip_to(2).unwrap();
  	assert_eq!(Ok(1), cur.peek_l());
  	let fix = cur.edit(zip::Dir::R, 2).unwrap();

  	// upzip back to a sequence to see the result
  	let restore = SeqZip::<_,_>::unzip(&fix);
  	let result = restore.iter().collect::<Vec<_>>();
  	assert_eq!(vec!(0,1,2,3), result);

  	// show off that this is a persistent structure
  	assert_eq!(Ok(7), save.peek_r());
  }
}
