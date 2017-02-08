use time::{Duration, precise_time_ns};
use rand::{StdRng,SeedableRng};
use eval::*;
use eval::eval_iraz::EvalIRaz;
use eval::eval_vec::EvalVec;

pub struct TestResult {
	edits: Vec<Duration>,
	full_edits: Vec<Duration>,
	computes: Vec<Duration>,
	full_computes: Vec<Duration>,
}

// TODO: Put this trait in a macro (for trait polymorphism)
// requiring name, edit, computation, additonal trait bounds
// let it be implemented to generate a test

trait Seq_Test<'a,G: ItemGen<Self::Item>,R> {
	type Item: Eval;
	fn run(self, p: &'a Params, data_gen: &G, rng: &mut StdRng) -> R;
}

impl<'a,E,S,G> Seq_Test<'a,G,TestResult> for S
where
	E: Eval + Ord,
	S: DataInit<'a,G,Item=E> + EditAppend + CompMax,
	G: ItemGen<E>,
{
	type Item = E;
	fn run(self, p: &'a Params, data_gen: &G, rng: &mut StdRng) -> TestResult {
		let mut testdata;
		let mut edit_result = None;
		let data = (*data_gen).clone();
		let mut e = EditParams{loc: 0, batch_size: p.edits};
		let mut len = p.start;
		let mut edits = Vec::with_capacity(p.changes);
		let mut full_edits = Vec::with_capacity(p.changes);
		let mut computes = Vec::with_capacity(p.changes);
		let mut full_computes = Vec::with_capacity(p.changes);

		// step 1: initialize sequence
		let full_init_time = Duration::span(||{
			edit_result = Some(S::init(p,data, rng));
		});
		let (init_time,dat) = edit_result.unwrap();
		edits.push(init_time);
		full_edits.push(full_init_time);
		testdata = dat;
		let mut comp_result = None;
		let full_comp_time = Duration::span(||{
			comp_result = Some(testdata.compute(rng));
		});
		let (comp_time,_) = comp_result.unwrap();
		computes.push(comp_time);
		full_computes.push(full_comp_time);

		// step 2: run a bunch of edits	
		for _ in 0..p.changes {
			// can't use span() here, because the closure would capture our testdata
			let mut accum_time = Duration::nanoseconds(0);
			let before = precise_time_ns();
			for _ in 0..p.batches {
				e.loc = rng.gen::<usize>() % ( len + 1 );
				len += e.batch_size;
				let (time,dat) = testdata.edit(&e, rng);
				accum_time = accum_time + time;
				testdata = dat;
			}
			let edit_full_time = Duration::nanoseconds((precise_time_ns() - before) as i64);
			edits.push(accum_time);
			full_edits.push(edit_full_time);
			let mut comp_result = None;
			let full_comp_time = Duration::span(||{
				comp_result = Some(testdata.compute(rng));
			});
			let (comp_time, _) = comp_result.unwrap();
			computes.push(comp_time);
			full_computes.push(full_comp_time);
		}

		TestResult {
			edits: edits,
			full_edits: full_edits,
			computes: computes,
			full_computes: full_computes,
		}
	}
}