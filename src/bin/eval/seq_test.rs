use time::{Duration};
use rand::{StdRng};
use eval::*;
use actions::*;

pub struct TestResult<D,A> {
	pub edits: Vec<Duration>,
	pub full_edits: Vec<Duration>,
	pub computes: Vec<Duration>,
	pub full_computes: Vec<Duration>,
	pub answers: Vec<A>,
	pub final_data: D,
}

pub struct FirstCrossover<G:Rng> {
	pub init: SizedSeq<G>,
	pub edit: BatchInsert,
	pub comp: FindMax,
}

impl<'a,D,G>
Testor<TestResult<D,D::Target>>
for FirstCrossover<G> where
	G:Rng,
	D:InitSeq<G>+EditInsert+CompMax,
{
	fn test(&mut self, rng: &mut StdRng) -> TestResult<D,D::Target> {
		let mut testdata;
		let mut edits = Vec::with_capacity(self.init.params.changes);
		let mut full_edits = Vec::with_capacity(self.init.params.changes);
		let mut computes = Vec::with_capacity(self.init.params.changes);
		let mut full_computes = Vec::with_capacity(self.init.params.changes);
		let mut answers = Vec::with_capacity(self.init.params.changes);

		// step 1: initialize sequence
		let mut edit_result = None;
		let full_init_time = Duration::span(||{
			edit_result = Some(self.init.create(rng));
		});
		let (init_time,dat) = edit_result.unwrap();
		edits.push(init_time);
		full_edits.push(full_init_time);
		testdata = dat;
		let mut comp_result = None;
		let full_comp_time = Duration::span(||{
			comp_result = Some(self.comp.compute(&testdata,rng));
		});
		let comp_time = comp_result.unwrap();
		computes.push(comp_time);
		//answers.push(answer);
		full_computes.push(full_comp_time);

		// step 2: run a bunch of edits	
		for _ in 0..self.init.params.changes {
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
			let comp_time = comp_result.unwrap();
			computes.push(comp_time);
			//answers.push(answer);
			full_computes.push(full_comp_time);
		}

		TestResult {
			edits: edits,
			full_edits: full_edits,
			computes: computes,
			full_computes: full_computes,
			answers: answers,
			final_data: testdata,
		}
	}
}