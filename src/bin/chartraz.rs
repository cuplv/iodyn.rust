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

use std::marker::PhantomData;
use rand::{StdRng,SeedableRng};
use time::{Duration};
use eval::*;
use eval::actions::*;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
use eval::seq_test::{TestResult,FirstCrossover};

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_TAG: &'static str = "None";
const DEFAULT_TAGHEAD: &'static str = "Tag";
const DEFAULT_START: usize = 0;
const DEFAULT_UNITSIZE: usize = 10;
const DEFAULT_NAMESIZE: usize = 1;
const DEFAULT_EDITS: usize = 1;
const DEFAULT_CHANGES: usize = 10;
const DEFAULT_TRIALS: usize = 10;
const DEFAULT_VARY: &'static str = "none";

enum WhichVary {
	Nil,S,U,N,E,C
}
use WhichVary::*;

fn main() {
  //command-line
  let args = clap::App::new("chartraz")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Produces comparison charts for RAZ data structure")
    .args_from_usage("\
      --nohead                  'supress header'
      --dataseed=[dataseed]			'seed for random data'
      --editseed=[edit_seed]    'seed for random edits (and misc.)'
      --tag=[tag]               'user tag'
      --taghead=[taghead]       'header title for tag'
      -s, --start=[start]       'starting sequence length'
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -t, --trials=[trials]     'trials to average over'
      --vary=[vary]             'parameter to vary (one of sunec, adjust x2)' ")
    .get_matches();
  let nohead = args.is_present("nohead");
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
  let tag = args.value_of("tag").unwrap_or(DEFAULT_TAG);
  let taghead = args.value_of("taghead").unwrap_or(DEFAULT_TAGHEAD);
	let start = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let unitsize = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namesize = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);
	let _vary = match args.value_of("vary").unwrap_or(DEFAULT_VARY) {
		"none"=>Nil,"s"=>S,"u"=>U,"n"=>N,"e"=>E,"c"=>C,
		_ => panic!("vary takes on of: s,u,n,e,c")
	};

	#[allow(unused)]
	let print_header = ||{
	   println!("Timestamp,Seed,SeqType,SeqNum,PriorElements,Insertions,Time,{}", taghead);
	};

	#[allow(unused)]
	let print_result = |version: &str, number: usize, prior_elms: usize, insertions: usize, time: Duration| {
		println!("{},{},{},{},{},{},{},{}",
			time::get_time().sec, dataseed, version, number, prior_elms, insertions, time, tag
		);
	};

  // print header
  if !nohead { print_header() }

  let mut test = FirstCrossover{
		init: SizedSeq{
			size: start,
			params: Params{
				start: start,
				unitsize: unitsize,
				namesize: namesize,
				edits: edits,
				changes: changes,
				trials: trials,
			},
			item_gen: StdRng::from_seed(&[dataseed]),
			phantom: PhantomData,
		},
		edit: BatchInsert(3),
		comp: FindMax,
  };

  let result_raz: TestResult<EvalIRaz<usize,StdRng>,Option<usize>> = test.test(&mut StdRng::from_seed(&[editseed]));
  let result_vec: TestResult<EvalVec<usize,StdRng>,Option<usize>> = test.test(&mut StdRng::from_seed(&[editseed]));

  //let answers: Vec<(usize,usize)> = result_raz.answers.iter().map(|d|d.unwrap()).zip(result_vec.answers.iter().map(|d|d.unwrap())).collect();
  let comp_raz: Vec<i64> = result_raz.computes.iter().map(|d|d.num_nanoseconds().unwrap()).collect();
  let comp_vec: Vec<i64> = result_vec.computes.iter().map(|d|d.num_nanoseconds().unwrap()).collect();
  let comp_both: Vec<(i64,i64)> = comp_raz.into_iter().zip(comp_vec.into_iter()).collect();
  let edit_raz: Vec<i64> = result_raz.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect();
  let edit_vec: Vec<i64> = result_vec.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect();
  let edit_both: Vec<(i64,i64)> = edit_raz.into_iter().zip(edit_vec.into_iter()).collect();
  
  println!("edits: {:?}", edit_both);
  println!("computes: {:?}", comp_both);
  //println!("answers: {:?}", answers);

}


