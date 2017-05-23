use std::marker::PhantomData;
use std::io::Write;
use time::{Duration};
use rand::{StdRng};
use stats::OnlineStats;
use actions::{Creator,Editor,Computor,Testor};

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
		let mut edits = Vec::with_capacity(self.changes+1);
		let mut full_edits = Vec::with_capacity(self.changes+1);
		let mut computes = Vec::with_capacity(self.changes+1);
		let mut full_computes = Vec::with_capacity(self.changes+1);

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

/// raw stats from the test
pub struct TestResult<I,O> {
	pub edits: Vec<Duration>,
	pub full_edits: Vec<Duration>,
	pub computes: Vec<Duration>,
	pub full_computes: Vec<Duration>,
	pub result_data: O,
	in_type: PhantomData<I>,
}

/// Cumulative aggregated results
pub struct TestResultAg {
	pub edits: Vec<f64>,
	pub edits_er: Vec<f64>,
	pub full_edits: Vec<f64>,
	pub full_edits_er: Vec<f64>,
	pub computes: Vec<f64>,
	pub computes_er: Vec<f64>,
	pub full_computes: Vec<f64>,
	pub full_computes_er: Vec<f64>,
}

fn as_milli(dur: &Vec<Duration>, cnt: usize) -> f64 {
	dur.iter().map(|d|{
		d.num_nanoseconds().unwrap() as f64 / 1_000_000.0
	}).take(cnt).sum()
}

pub fn aggregate<I,O>(data: &[TestResult<I,O>]) -> TestResultAg {
	if data.is_empty() { panic!("no data") }
	let mut result = TestResultAg{
		edits: Vec::new(),
		edits_er: Vec::new(),
		full_edits: Vec::new(),
		full_edits_er: Vec::new(),
		computes: Vec::new(),
		computes_er: Vec::new(),
		full_computes: Vec::new(),
		full_computes_er: Vec::new(),
	};
	let samples = data.len();
	let samp_sqrt = (samples as f64).sqrt();
	let changes = data[0].edits.len();
	for c in 0..changes {
		let mut edits = OnlineStats::new();
		let mut full_edits = OnlineStats::new();
		let mut computes = OnlineStats::new();
		let mut full_computes = OnlineStats::new();
		for s in 0..samples {
			edits.add(as_milli(&data[s].edits,c+1));
			full_edits.add(as_milli(&data[s].full_edits,c+1));
			computes.add(as_milli(&data[s].computes,c+1));
			full_computes.add(as_milli(&data[s].full_computes,c+1));
		}
		result.edits.push(edits.mean());
		result.edits_er.push(edits.stddev()/samp_sqrt);
		result.full_edits.push(full_edits.mean());
		result.full_edits_er.push(full_edits.stddev()/samp_sqrt);
		result.computes.push(computes.mean());
		result.computes_er.push(computes.stddev()/samp_sqrt);
		result.full_computes.push(full_computes.mean());
		result.full_computes_er.push(full_computes.stddev()/samp_sqrt);
	}
	result
}

impl TestResultAg {
	pub fn write_to(&self, dat: &mut Write) {
		writeln!(dat,"# '{}'\t'{}'\t'{}'\t'{}'\t'{}'\t'{}'\t'{}'\t'{}'\t'{}'",
			"Changes",
			"Edit Time",
			"Edit Standard Error",
			"Full Edit Time",
			"Full Edit Standard Error",
			"Compute Time",
			"Compute Standard Error",
			"Full Compute Time",
			"Full Compute Standard Error"
		).unwrap();
		for i in 0..self.edits.len() {
			writeln!(dat,"{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
				i,
				self.edits[i],
				self.edits_er[i],
				self.full_edits[i],
				self.full_edits_er[i],
				self.computes[i],
				self.computes_er[i],
				self.full_computes[i],
				self.full_computes_er[i]
			).unwrap();
		}
	}
}

pub struct TestMResult<I,O> {
	pub edits: Vec<Duration>,
	pub full_edits: Vec<Duration>,
	pub computes: Vec<Vec<Duration>>,
	pub full_computes: Vec<Duration>,
	pub result_data: O,
	in_type: PhantomData<I>,
}
use std::fmt::Debug;
impl<C,E,U,I:Debug,O:Debug>
Testor<TestMResult<I,O>>
for EditComputeSequence<C,E,U> where
	C: Creator<Duration,I>,
	E: Editor<Duration,I>,
	U: Computor<(Vec<Duration>,O),I>,
{
	fn test(&mut self, rng: &mut StdRng) -> TestMResult<I,O> {
		let mut testdata: I;
		let mut results: O;
		let mut edits = Vec::with_capacity(self.changes+1);
		let mut full_edits = Vec::with_capacity(self.changes+1);
		let mut computes = Vec::with_capacity(self.changes+1);
		let mut full_computes = Vec::with_capacity(self.changes+1);

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
		let (comp_times,result) = comp_result.unwrap();
		//println!("first result: {:?}", result);
		results = result;
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
			let (comp_times,result) = comp_result.unwrap();
			//println!("result({}): {:?}", _i, result);
			results = result;
			computes.push(comp_times);
			full_computes.push(full_comp_time);
		}

		TestMResult {
			edits: edits,
			full_edits: full_edits,
			computes: computes,
			full_computes: full_computes,
			result_data: results,
			in_type: PhantomData,
		}
	}
}
