//! Conversions by Memoization
//!

/// Construct `Self` via a memoized conversion
///
/// Rerunning `memo_from` on a slightly modified `T` is expected to take
/// asymptotically less time than the initial run, but with some constant
/// overhead on the initial run
pub trait MemoFrom<T> {
	fn memo_from(&T) -> Self;
}

/// Convert `self` via a memoized conversion
///
/// similar to MemoFrom
pub trait MemoInto<T> {
	fn memo_into(&self) -> T;
}

impl<T: Clone, U:MemoFrom<T>>
MemoInto<U> for T {
	fn memo_into(&self) -> U {
		U::memo_from(self)
	}
}

impl<T: Clone> MemoFrom<T> for T {
	fn memo_from(t: &T) -> T { t.clone() } 
}

