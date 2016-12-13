// Trait for a zipper, a cursor in a sequence
// Editting a zipper is assumed to be efficient (O(1))

pub trait Zip<T>: Sized {
	// required fn's
	fn zip_l(&self) -> Result<Self,&str>;
	fn zip_r(&self) -> Result<Self,&str>;
	fn peek_l(&self) -> Result<&T,&str>;
	fn peek_r(&self) -> Result<&T,&str>;
	fn push_l(&self, val: T) -> Self;
	fn push_r(&self, val: T) -> Self;
	fn pull_l(&self) -> Result<Self,&str>;
	fn pull_r(&self) -> Result<Self,&str>;

	// additional derived fn's
	fn edit_l(&self, val: T) -> Result<Self,&str> {
		match self.pull_l() {
			Ok(zip) => Ok(zip.push_l(val)),
			Err(_) => Err("edit past beginning of sequence")
		}
	}
	fn edit_r(&self, val: T) -> Result<Self,&str> {
		match self.pull_r() {
			Ok(zip) => Ok(zip.push_r(val)),
			Err(_) => Err("edit past end of sequence")
		}
	}
	fn pop_l(&self) -> Result<(&T,Self),&str> {
		match self.peek_l() {
			Ok(val) => Ok((val,self.pull_l().unwrap())),
			Err(_) => Err("pop past beginning of sequence")
		}
	}
	fn pop_r(&self) -> Result<(&T,Self),&str> {
		match self.peek_r() {
			Ok(val) => Ok((val,self.pull_r().unwrap())),
			Err(_) => Err("pop past end of sequence")
		}
	}

	// direction abstracted
	fn zip(&self, dir: Dir) -> Result<Self,&str> {
		match dir {Dir::L => self.zip_l(), Dir::R => self.zip_r()} 
	}
	fn peek(&self, dir: Dir) -> Result<&T,&str> {
		match dir {Dir::L => self.peek_l(), Dir::R => self.peek_r()}
	}
	fn push(&self, dir: Dir, val: T) -> Self {
		match dir {Dir::L => self.push_l(val), Dir::R => self.push_r(val)}
	}
	fn pull(&self, dir: Dir) -> Result<Self,&str> {
		match dir {Dir::L => self.pull_l(), Dir::R => self.pull_r()}
	}
	fn edit(&self, dir: Dir, val: T) -> Result<Self,&str> {
		match dir {Dir::L => self.edit_l(val), Dir::R => self.edit_r(val)}
	}
	fn pop(&self, dir: Dir) -> Result<(&T,Self),&str> {
		match dir {Dir::L => self.pop_l(), Dir::R => self.pop_r()}
	}

	// command abstracted for Self return values
	fn do_cmd(&self, cmd: Cmd<T>) -> Result<Self,&str> {
		match cmd {
			Cmd::Zip(dir) => self.zip(dir),
			Cmd::Push(dir, val) => Ok(self.push(dir, val)),
			Cmd::Edit(dir, val) => self.edit(dir, val),
			Cmd::Pull(dir) => self.pull(dir)
		}
	}

}

// structs for zip fn abstractions

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
pub enum Dir {L, R}

impl Dir {
	pub fn rev(self) -> Self {
		match self {Dir::R => Dir::L, Dir::L => Dir::R}
	}
}

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
pub enum Cmd<T> {Zip(Dir), Push(Dir,T), Edit(Dir,T), Pull(Dir)}





// Implement with stack

use stack::Stack;

#[derive(Clone)]
pub struct Stacks<T> { l: Stack<T>, r: Stack<T> }

impl<T> Stacks<T> {
	// constructors
	pub fn new() -> Stacks<T> {
		Stacks { l: Stack::new(), r: Stack::new() }
	}
	pub fn at_left(right: Stack<T>) -> Stacks<T> {
		Stacks { l: Stack::new(), r: right }
	}
	pub fn at_right(left: Stack<T>) -> Stacks<T> {
		Stacks { l: left, r: Stack::new() }
	}
	pub fn between(left: Stack<T>, right: Stack<T>) -> Stacks<T>  {
		Stacks { l: left, r: right }
	}

	// inspectors
	pub fn left_stack(&self) -> Stack<T> { self.l.clone() }
	pub fn right_stack(&self) -> Stack<T> { self.r.clone() }
}

impl<T: Clone> Zip<T> for Stacks<T> {

	fn zip_l(&self) -> Result<Self,&str> {
		if let Some(head) = self.l.peek() {
			Ok( Stacks {l: self.l.pull().unwrap(), r: self.r.push(head.clone())})
		} else { Err("Stacks: Move past beginning of sequence") }
	}
	fn zip_r(&self) -> Result<Self,&str> {
		if let Some(head) = self.r.peek() {
			Ok( Stacks {l: self.l.push(head.clone()), r: self.r.pull().unwrap()})
		} else { Err("Stacks: Move past end of sequence") }
	}

	fn peek_l(&self) -> Result<&T,&str> {
		if let Some(head) = self.l.peek() { Ok(head) }
		else { Err("Stacks: Peek past beginning of sequence")}
	}
	fn peek_r(&self) -> Result<&T,&str> {
		if let Some(head) = self.r.peek() { Ok(head) }
		else { Err("Stacks: Peek past end of sequence")}
	}

	fn push_l(&self, val: T) -> Self {
		Stacks {l: self.l.push(val), r: self.r.clone()}
	}
	fn push_r(&self, val: T) -> Self {
		Stacks {l: self.l.clone(), r: self.r.push(val)}
	}

	fn pull_l(&self) -> Result<Self,&str> {
		if let Some(tail) = self.l.pull() {
			Ok(Stacks { l: tail, r: self.r.clone() })
		} else { Err("Stacks: Pull past beginning of sequence")}
	}
	fn pull_r(&self) -> Result<Self,&str> {
		if let Some(tail) = self.r.pull() {
			Ok(Stacks { l: self.l.clone(), r: tail })
		} else { Err("Stacks: Pull past end of sequence")}
	}

}

