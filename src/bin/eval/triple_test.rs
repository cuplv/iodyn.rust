use time::Duration;
use rand::{StdRng,SeedableRng};
use Params;
use eval::*;
use eval::eval_iraz::EvalIRaz;
use eval::eval_vec::EvalVec;

struct TripleTest<I,N> 
{
	incremental: I,
	naive: I,
	noninc: N,
}


impl Tester for TripleTest<EvalIRaz<usize,StdRng>,EvalVec<usize,StdRng>> {
	fn init(&mut self, p: &Params, mut rng: &mut StdRng) -> Vec<Duration> {
		let mut raztree = None;
		let mut nraztree = None;
		let mut vec = None;
		let data = StdRng::from_seed(&p.dataseed);
		let inc_gen_time = Duration::span(||{
			raztree = Some(EvalIRaz::init(p,data.clone(), &mut rng));
		});
		let naive_gen_time = Duration::span(||{
			nraztree = Some(EvalIRaz::init(p,data.clone(), &mut rng));
		});
		let non_gen_time = Duration::span(||{
			vec = Some(EvalVec::init(p,data.clone(), &mut rng));
		});
		*self = TripleTest{
			incremental: raztree.unwrap(),
			naive: nraztree.unwrap(),
			noninc: vec.unwrap(),
		};
		vec![non_gen_time,naive_gen_time,inc_gen_time]
	}
	fn edit(&mut self, _p: &Params, _rng: &mut StdRng) -> Vec<Duration> {
		unimplemented!();
	}
	fn run(&mut self, _p: &Params, _rng: &mut StdRng) -> Vec<Duration> {
		unimplemented!();		
	}
}

