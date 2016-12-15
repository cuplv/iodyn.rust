extern crate rand;

pub mod zip;
pub mod stack;
pub mod seqzip;
pub mod raz;

#[cfg(test)]
mod tests {
	use std::rc::Rc;
	use super::*;
	use seqzip::{AtLeft,AtRight};
	use zip::Zip;
	use seqzip::{Seq, SeqZip};

  #[test]
  fn it_works() {
  	// define a sequence
  	let none = stack::Stack::new();
  	let some = none.push(Rc::new(3)).push(Rc::new(7)).push(Rc::new(1)).push(Rc::new(0));
  	{
  		// iter is borrwoing the value; keep it local
	  	let result = some.iter().collect::<Vec<_>>();
	  	assert_eq!(vec!(Rc::new(0),Rc::new(1),Rc::new(7),Rc::new(3)), result);
  	}

  	// save some of it for later
  	let save = some.pull().unwrap().pull().unwrap();

  	// use a zip to edit it
  	let cur = AtLeft(some).zip_to(2).unwrap();
  	assert_eq!(Ok(Rc::new(1)), cur.peek_l());
  	let fix = cur.edit(zip::Dir::R, Rc::new(2)).unwrap();

  	// upzip back to a sequence to see the result
  	let restore = SeqZip::<_,AtLeft<_>>::unzip(&fix);
  	let result = restore.iter().collect::<Vec<_>>();
  	assert_eq!(vec!(Rc::new(0),Rc::new(1),Rc::new(2),Rc::new(3)), result);

  	// unzip the other way the other way
  	let back: AtRight<_> = fix.unzip();
  	let result: Vec<_> = back.iter().collect();
  	assert_eq!(vec!(Rc::new(3),Rc::new(2),Rc::new(1),Rc::new(0)), result);

  	// show off that this is a persistent structure
  	assert_eq!(Some(Rc::new(7)), save.peek());
  }
}
