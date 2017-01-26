//! Traits for various flavors of persistent trees

use std::fmt::Debug;
use std::hash::Hash;

use rand::{Rng, Rand};
use adapton::engine::Name;

/// General trait for a persistent binary tree
///
/// All nodes are trees themselves, and
/// all trees are immutable and shared.
/// An owned tree is really a link to a
/// shared node.
pub trait BinTree<T> where Self: Sized {
	/// construct a new tree with existing nodes
	#[allow(unused_variables)]
	fn bin_make(
		data: T,
		l_branch: Option<Self>,
		r_branch: Option<Self>
	) -> Self { panic!("This constructor is unavailable for this type. Use a type specific one instead.")}

	/// get the left branch
	fn l_tree(&self) -> Option<Self>;

	/// get the right branch
	fn r_tree(&self) -> Option<Self>;

	/// get a reference to the data in this branch
	fn peek(&self) -> &T;

	/// Perform a calculation recursively over all
	/// branches.
	///
	/// The function `node_calc` takes the data from
	/// the current branch and the result of a
	/// recursive call to the lower branches, if they
	/// exist.
	fn fold_up<R,F>(&self, node_calc: &mut F) -> R
	where F: FnMut(&T,Option<R>,Option<R>) -> R;

}

/// Levels for the LevelTree
///
/// Each type should have its own random distribution.
/// Wrappers of primitive integers would work well.
pub trait Level: Ord+Copy+Rand {
	/// construct greatest `Level`
	fn l_max() -> Self;
	/// construct least `Level`
	fn l_min() -> Self;
	/// access the next greater `Level`
	fn l_inc(self) -> Self;
	/// access the next lesser `Level`
	fn l_dec(self) -> Self;
}

/// Wrapper around u8 to generate random `Level`s
/// appropriate for a balanced binary tree. 
///
/// If there is a distinction between "leaves" and
/// "branches", use `l_min()` for leaves and `Rng::gen()`
/// for branches.
#[derive(PartialEq,Eq,PartialOrd,Ord,Clone,Copy)]
pub struct NegBin(u8);
impl Rand for NegBin {
	/// Generates Levels 1-64 from a negative binomial
	/// distribution. This is appropriate for binary trees
	/// of up to at least 2^64 elements. 
	fn rand<R: Rng>(rng: &mut R) -> Self {
		let num = rng.gen::<u64>();
		let lev = (num << 1).trailing_zeros() as u8;
		NegBin(lev)
	}
}
impl Level for NegBin {
	fn l_max() -> Self { NegBin(u8::max_value()) }
	fn l_min() -> Self { NegBin(u8::min_value()) }
	fn l_inc(self) -> Self { NegBin(self.0+1) }
	fn l_dec(self) -> Self { NegBin(self.0-1) }
}

/// A binary tree with "levels" for each node
///
/// Trees representing sequences that have the same length 
/// and have the same levels will have the same structure,
/// regaurdless of the order of operations. Greater levels
/// will appear closer to the root than lesser levels.
pub trait LevelTree<L: Level, T>: BinTree<T> {
	/// construct a new tree
	///
	/// This should return `None` if the levels are
	/// inappropriate. Left-branch levels must be lesser than
	/// the root and right-branch levels must not be greater. 
	#[allow(unused_variables)]
	fn lev_make(
		level: L,
		data: T,
		l_branch: Option<Self>,
		r_branch: Option<Self>
	) -> Option<Self> { panic!("This constructor is unavailable for this type. Use a type specific one instead.")}

	/// get the level of this node
	fn level(&self) -> L;
}

pub trait NominalTree<L: Level, T: >: LevelTree<L,T> 
where T: Clone+Debug+Eq+Hash
{
	/// construct a new tree
	///
	/// See `adapton` crate for uses of names.
	/// See `LevelTree` in this mod for use of levels.
	#[allow(unused_variables)]
	fn nm_make(
		name: Name,
		level: L,
		data: T,
		l_branch: Option<Self>,
		r_branch: Option<Self>
	) -> Option<Self> { panic!("This constructor is unavailable for this type. Use a type specific one instead.")}

	/// get the incremental name from this node
	fn name(&self) -> Name; 
}
