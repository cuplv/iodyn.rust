use std::fmt::{self,Debug};
use std::rc::Rc;
use rand::{StdRng,Rng,Rand};
use time::Duration;
use pmfp_collections::inc_archive_stack::AStack as IAStack;
use primitives::*;
use adapton::engine::Name;
use interface::{Adapt};

/// Test harness for `IAStack`
///
/// Coordinates elements and insertion position
#[derive(Clone)]
pub struct EvalIAStack<E:Adapt,G:Rng> {
	vec: IAStack<E,u32>,
	coord: G,
}
impl<E:Adapt,G:Rng>
EvalIAStack<E,G> {
	fn new(coord:G) -> Self {
		EvalIAStack {
			vec: IAStack::new(),
			coord: coord,
		}
	}
}

impl<E:Adapt,G:Rng+Clone>
CreateEmpty<G> for EvalIAStack<E,G>{
	fn inc_empty(_unitgauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalIAStack::new((*coord).clone()));
		});
		(time,eval.unwrap())
	}
}
