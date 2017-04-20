use std::fmt::{self,Debug};
use rand::{StdRng,Rng};
use time::Duration;
use pmfp_collections::inc_archive_stack::AStack as IAStack;
use primitives::*;
use interface::{Adapt};

/// Test harness for `IAStack`
///
/// Coordinates elements and insertion position
#[derive(Clone)]
pub struct EvalIAStack<E:Adapt,G:Rng> {
	stack: IAStack<E,u32>,
	coord: G,
}
impl<E:Adapt,G:Rng> Debug for EvalIAStack<E,G> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{:?}",self.stack.clone().into_iter().collect::<Vec<_>>())
	}
}
impl<E:Adapt,G:Rng>
EvalIAStack<E,G> {
	fn new(coord:G) -> Self {
		EvalIAStack {
			stack: IAStack::new(),
			coord: coord,
		}
	}
}

impl<E:Adapt,G:Rng+Clone>
CreateEmpty<G> for EvalIAStack<E,G>{
	fn inc_empty(_datagauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalIAStack::new((*coord).clone()));
		});
		(time,eval.unwrap())
	}
}
