//! This is a system for creating charts of the
//! performance of various forms of the raz
//! data structures defined in this crate

extern crate rand;
extern crate time;
#[macro_use] extern crate clap;
extern crate stats;
extern crate adapton;
extern crate pmfp_collections;

mod eval;

use rand::{StdRng,SeedableRng};
use eval::*;
use eval::actions::*;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
use eval::seq_test::{TestResult,FirstCrossover};
use adapton::engine::manage::*;

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_START: usize = 10000;
const DEFAULT_UNITSIZE: usize = 10;
const DEFAULT_NAMESIZE: usize = 1;
const DEFAULT_EDITS: usize = 1;
const DEFAULT_CHANGES: usize = 10;
const DEFAULT_TRIALS: usize = 10;

fn main() {
  //command-line
  let args = clap::App::new("chartraz")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Produces comparison charts for RAZ data structure")
    .args_from_usage("\
      --dataseed=[dataseed]			'seed for random data'
      --editseed=[edit_seed]    'seed for random edits (and misc.)'
      -s, --start=[start]       'starting sequence length'
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -t, --trials=[trials]     'trials to average over' ")
    .get_matches();
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
	let start = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let unitsize = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namesize = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);

  let mut test = FirstCrossover{
		init: IncrementalInit {
			size: start,
      unitgauge: unitsize,
      namegauge: namesize,
			coord: StdRng::from_seed(&[dataseed]),
		},
		edit: BatchInsert(edits),
		comp: FindMax,
    changes: changes,
  };

  let _ = init_dcg(); assert!(engine_is_dcg());

  let mut rng = StdRng::from_seed(&[editseed]);
  let result_raz: TestResult<EvalIRaz<usize,StdRng>,Option<usize>> = test.test(&mut rng);
  let result_vec: TestResult<EvalVec<usize,StdRng>,Option<usize>> = test.test(&mut rng);

  //let answers: Vec<(usize,usize)> = result_raz.answers.iter().map(|d|d.unwrap()).zip(result_vec.answers.iter().map(|d|d.unwrap())).collect();
  let comp_raz = result_raz.computes.iter().map(|d|d.num_nanoseconds().unwrap());
  let comp_vec = result_vec.computes.iter().map(|d|d.num_nanoseconds().unwrap());
  let comp_both: Vec<(i64,i64)> = comp_raz.zip(comp_vec).collect();
  let edit_raz = result_raz.edits.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_vec = result_vec.edits.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_both: Vec<(i64,i64)> = edit_raz.zip(edit_vec).collect();
  
  println!("edits: {:?}", edit_both);
  println!("computes: {:?}", comp_both);
  //println!("answers: {:?}", answers);

}


