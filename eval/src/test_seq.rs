use std::marker::PhantomData;
use time::{Duration};
use rand::{StdRng};
use actions::{Creator,Editor,Computor,Testor};

pub struct TestResult<I,O> {
	pub edits: Vec<Duration>,
	pub full_edits: Vec<Duration>,
	pub computes: Vec<Duration>,
	pub full_computes: Vec<Duration>,
	pub result_data: O,
	in_type: PhantomData<I>,
}

pub struct EditComputeSequence<C,E,U> {
	pub init: C,
	pub edit: E,
	pub comp: U,
	pub changes: usize,
}
impl<'a,C,E,U,I,O>
Testor<TestResult<I,O>>
for EditComputeSequence<C,E,U> where
	C: Creator<Duration,I>,
	E: Editor<Duration,I>,
	U: Computor<(Duration,O),I>,
{
	fn test(&mut self, rng: &mut StdRng) -> TestResult<I,O> {
		let mut testdata: I;
		let mut results: O;
		let mut edits = Vec::with_capacity(self.changes);
		let mut full_edits = Vec::with_capacity(self.changes);
		let mut computes = Vec::with_capacity(self.changes);
		let mut full_computes = Vec::with_capacity(self.changes);

		// step 1: initialize sequence
		let mut init_result = None;
		let full_init_time = Duration::span(||{
			init_result = Some(self.init.create(rng));
		});
		let (init_time,dat) = init_result.unwrap();
		edits.push(init_time);
		full_edits.push(full_init_time);
		testdata = dat;
		let mut comp_result = None;
		let full_comp_time = Duration::span(||{
			comp_result = Some(self.comp.compute(&testdata,rng));
		});
		let (comp_time, result) = comp_result.unwrap();
		results = result;
		computes.push(comp_time);
		full_computes.push(full_comp_time);

		// step 2: run a bunch of edits	
		for _ in 0..self.changes {
			let mut edit_result = None;
			let edit_full_time = Duration::span(||{
				edit_result = Some(self.edit.edit(testdata,rng));
			});
			let (edit_time,dat) = edit_result.unwrap();
			edits.push(edit_time);
			full_edits.push(edit_full_time);
			testdata = dat;
			let mut comp_result = None;
			let full_comp_time = Duration::span(||{
				comp_result = Some(self.comp.compute(&testdata,rng));
			});
			let (comp_time, result) = comp_result.unwrap();
			results = result;
			computes.push(comp_time);
			full_computes.push(full_comp_time);
		}

		TestResult {
			edits: edits,
			full_edits: full_edits,
			computes: computes,
			full_computes: full_computes,
			result_data: results,
			in_type: PhantomData,
		}
	}
}

pub struct TestMResult<I,O> {
	pub edits: Vec<Duration>,
	pub full_edits: Vec<Duration>,
	pub computes: Vec<Vec<Duration>>,
	pub full_computes: Vec<Duration>,
	in_type: PhantomData<I>,
	out_type: PhantomData<O>,
}
//use std::fmt::Debug;
impl<C,E,U,I,O>
Testor<TestMResult<I,O>>
for EditComputeSequence<C,E,U> where
	C: Creator<Duration,I>,
	E: Editor<Duration,I>,
	U: Computor<(Vec<Duration>,O),I>,
{
	fn test(&mut self, rng: &mut StdRng) -> TestMResult<I,O> {
		let mut testdata: I;
		let mut edits = Vec::with_capacity(self.changes);
		let mut full_edits = Vec::with_capacity(self.changes);
		let mut computes = Vec::with_capacity(self.changes);
		let mut full_computes = Vec::with_capacity(self.changes);

		// step 1: initialize sequence
		let mut init_result = None;
		let full_init_time = Duration::span(||{
			init_result = Some(self.init.create(rng));
		});
		let (init_time,dat) = init_result.unwrap();
		//println!("created: {:?}", dat);
		edits.push(init_time);
		full_edits.push(full_init_time);
		testdata = dat;
		let mut comp_result = None;
		let full_comp_time = Duration::span(||{
			comp_result = Some(self.comp.compute(&testdata,rng));
		});
		let (comp_times,_result) = comp_result.unwrap();
		//println!("first result: {:?}", _result);
		computes.push(comp_times);
		full_computes.push(full_comp_time);

		// step 2: run a bunch of edits	
		for _i in 0..self.changes {
			let mut edit_result = None;
			let edit_full_time = Duration::span(||{
				edit_result = Some(self.edit.edit(testdata,rng));
			});
			let (edit_time,dat) = edit_result.unwrap();
			//println!("edit({}): {:?}", _i, dat);
			edits.push(edit_time);
			full_edits.push(edit_full_time);
			testdata = dat;
			let mut comp_result = None;
			let full_comp_time = Duration::span(||{
				comp_result = Some(self.comp.compute(&testdata,rng));
			});
			let (comp_times,_result) = comp_result.unwrap();
			//println!("result({}): {:?}", _i, _result);
			computes.push(comp_times);
			full_computes.push(full_comp_time);
		}

		TestMResult {
			edits: edits,
			full_edits: full_edits,
			computes: computes,
			full_computes: full_computes,
			in_type: PhantomData,
			out_type: PhantomData,
		}
	}
}