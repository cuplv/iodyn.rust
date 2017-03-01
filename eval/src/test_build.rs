use std::marker::PhantomData;
use time::{Duration};
use rand::{Rng,StdRng};
use primitives::*;
use actions::{Creator,Testor,IncrementalInit};

pub struct TestResult<D> {
	pub build: Vec<Duration>,
	//pub finalize: Vec<Duration>,
	pub build_full: Vec<Duration>,
	pd: PhantomData<D>,
}

pub struct BuildTest<G:Rng> {
	pub init: IncrementalInit<G>,
	pub multiplier: f32,
	pub count: usize,
}
impl<'a,D,G:Rng+Clone>
Testor<TestResult<D>>
for BuildTest<G> where 
	D: CreateInc<G>,
{
	fn test(&mut self, rng: &mut StdRng) -> TestResult<D> {
		let mut build = Vec::with_capacity(self.count);
		//let mut finalize = Vec::with_capacity(self.changes);
		let mut build_full = Vec::with_capacity(self.count);

		for i in 0..self.count {
			let size = self.multiplier.powi(i as i32) as usize;
			// copy initial state to reuse for each iteration
			let mut builder = IncrementalInit{size: size, ..self.init.clone()};
			let mut time = None;
			let full_time = Duration::span(||{
				time = Some(builder.create(rng));
			});
			let (time,_):(Duration,D) = time.unwrap();
			build.push(time);
			build_full.push(full_time);
		}

		TestResult {
			build: build,
			build_full: build_full,
			pd: PhantomData,
		}
	}
}