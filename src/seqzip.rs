// Trait for a sequence-zipper pair, a sequence that can be converted
// to and from a zipper at different locations.
// This conversion is assumed to be efficient (O(log n) or better)

use zip::Zip;
use std::ops::Deref;

pub trait Seq<T,Z>: Sized where Z: SeqZip<T,Self> {
	fn zip_to(&self, loc: usize) -> Result<Z,&str>;
}

pub trait SeqZip<T,S>: Zip<T> where S: Seq<T,Self> {
	fn unzip(&self) -> S;
}





// implement with Stack for testing (inefficent: O(n))

use stack::Stack;
use zip::Stacks;

// Marker types for interpreting the stack (deref'd for convenience)
#[derive(Clone,Debug)]
pub struct AtLeft<T>(pub Stack<T>);
#[derive(Clone,Debug)]
pub struct AtRight<T>(pub Stack<T>);

impl<T> Deref for AtLeft<T> {
	type Target = Stack<T>;
	fn deref(&self) -> &Self::Target {
		let AtLeft(ref stack) = *self;
		stack
	}
}
impl<T> Deref for AtRight<T> {
	type Target = Stack<T>;
	fn deref(&self) -> &Self::Target {
		let AtRight(ref stack) = *self;
		stack
	}
}

impl<T: Clone> Seq<T, Stacks<T>> for AtLeft<T> {
	fn zip_to(&self, loc: usize) -> Result<Stacks<T>,&str> {
		let mut zip = Stacks::at_left((**self).clone());
		let mut pos = 0;
		while loc > pos {
	    zip = match zip.zip_r() {
	    	Ok(z) => z,
      	Err(_) => { return Err("Stack: Zip past end of sequence") }
      };
      pos += 1;
	  }
		Ok(zip)
	}
}

impl<T: Clone> Seq<T, Stacks<T>> for AtRight<T> {
	fn zip_to(&self, loc: usize) -> Result<Stacks<T>,&str> {
		let mut zip = Stacks::at_right((**self).clone());
		let mut size = 0;
		while let Ok(leftward) = zip.zip_l() { zip = leftward; size += 1; };
		if size < loc { return Err("Stack: Zip past end of sequence") };
		for _ in 0..loc {
	    zip = zip.zip_r().unwrap();
	  }
		Ok(zip)
	}
}

impl<T: Clone> SeqZip<T, AtLeft<T>> for Stacks<T> {
	fn unzip(&self) -> AtLeft<T> {
		let mut zip = self.clone();
		while let Ok(leftward) = zip.zip_l() { zip = leftward };
		AtLeft(zip.right_stack())
	}
}

impl<T: Clone> SeqZip<T, AtRight<T>> for Stacks<T> {
	fn unzip(&self) -> AtRight<T> {
		let mut zip = self.clone();
		while let Ok(rightward) = zip.zip_r() { zip = rightward };
		AtRight(zip.left_stack())
	}
}

