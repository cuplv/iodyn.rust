//! Non-Incremental tests

use std::marker::PhantomData;
use time::{Duration};
use rand::{Rng,StdRng};
use primitives::*;
use actions::{Creator,Editor,Testor,IncrementalInit,BatchInsert};

pub struct TestResult<D> {
	pub build: Vec<Duration>,
	pub build_full: Vec<Duration>,
	pub edit: Vec<Duration>,
	pub edit_full: Vec<Duration>,
	pd: PhantomData<D>,
}

pub struct BuildTest<G:Rng> {
	pub init: IncrementalInit<G>,
	pub edit: BatchInsert,
	pub multiplier: f32,
	pub count: usize,
}
impl<'a,D,G:Rng+Clone>
Testor<TestResult<D>>
for BuildTest<G> where 
	D: EditInsert+CreateInc<G>,
{
	fn test(&mut self, rng: &mut StdRng) -> TestResult<D> {
		let mut build = Vec::with_capacity(self.count);
		let mut build_full = Vec::with_capacity(self.count);
		let mut edit = Vec::with_capacity(self.count);
		let mut edit_full = Vec::with_capacity(self.count);

		for i in 0..self.count {
			let size = self.multiplier.powi(i as i32) as usize;
			// copy initial state to reuse for each iteration
			let mut builder = IncrementalInit{size: size, ..self.init.clone()};
			let mut time = None;
			let full_time = Duration::span(||{
				time = Some(builder.create(rng));
			});
			let (build_time,data):(Duration,D) = time.unwrap();
			build.push(build_time);
			build_full.push(full_time);
			time = None;
			let full_time = Duration::span(||{
				time = Some(self.edit.edit(data,rng));
			});
			let (edit_time,_):(Duration,D) = time.unwrap();
			edit.push(edit_time);
			edit_full.push(full_time);
		}

		TestResult {
			build: build,
			build_full: build_full,
			edit: edit,
			edit_full: edit_full,
			pd: PhantomData,
		}
	}
}