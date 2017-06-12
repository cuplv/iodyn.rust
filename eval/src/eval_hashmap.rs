use std::fmt::{self,Debug};
use std::rc::Rc;
use rand::{StdRng,Rng,Rand};
use time::Duration;
use std::collections::HashMap;
use primitives::*;

/// Test harness for `HashMap`
///
/// Coordinates elements and insertion position
#[derive(Clone)]
pub struct EvalHashMap<E,G:Rng> {
	map: HashMap<usize, E>,
	coord: G,
}
impl<E:Debug,G:Rng> Debug for EvalHashMap<E,G> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{:?}",self.map)
	}
}
impl<E,G:Rng>
EvalHashMap<E,G> {
	fn new(coord:G) -> Self {
		EvalHashMap {
			map: HashMap::new(),
			coord: coord,
		}
	}
}

impl<E,G:Rng+Clone>
CreateEmpty<G> for EvalHashMap<E,G>{
	fn inc_empty(_datagauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalHashMap::new((*coord).clone()));
		});
		(time,eval.unwrap())
	}
}

impl<E,G:Rng+Clone>
CreateFrom<HashMap<usize, E>,G> for EvalHashMap<E,G>{
	fn inc_from(data: HashMap<usize, E>, _datagauge: usize, _namegauge: usize, coord: &G, _rng: &mut StdRng) -> (Duration, Self) {
		let mut eval = None;
		let time = Duration::span(||{
			eval = Some(EvalHashMap{map:data,coord:(*coord).clone()});
		});
		(time,eval.unwrap())
	}
}

impl<E:Rand,G:Rng+Clone>
CreateInc<G>
for EvalHashMap<E,G> {
	fn inc_init(size: usize, _datagauge: usize, _namegauge: usize, coord: &G, mut _rng: &mut StdRng) -> (Duration,Self) {
		let mut eval = EvalHashMap::new((*coord).clone());
		let data_iter = eval.coord.gen_iter::<E>().take(size).collect::<Vec<_>>().into_iter();
		let mut pos = 1;
		let time = Duration::span(||{
			for dat in data_iter {
				eval.map.insert(pos, dat);
				pos = pos + 1;
			}
		});
		(time,eval)
	}
}

impl<E:Rand,G:Rng>
EditInsert for EvalHashMap<E,G> {
	fn insert(mut self, batch_size: usize, _rng: &mut StdRng) -> (Duration,Self) {
		let loc = self.coord.gen::<usize>() % self.map.capacity();
		let data_vec = self.coord.gen_iter().take(batch_size).collect::<Vec<_>>().into_iter();
		let time = Duration::span(||{
			for val in data_vec {
				self.map.entry(loc).or_insert(val);
			}
		});
		(time,self)
	}
}

impl<E:Clone,O,G:Rng>
CompNative<O> for EvalHashMap<E,G> {
	type Input = HashMap<usize, E>;
	fn comp_native<F>(&self, f:Rc<F>, _rng: &mut StdRng) -> (Duration,O) where
		F:Fn(&Self::Input)->O,
	{
		let mut nat = None;
		let time = Duration::span(||{
			nat = Some(f(&self.map));
		});
		(time, nat.unwrap())
	}
}