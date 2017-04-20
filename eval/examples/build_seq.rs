//! This is a system for creating charts of the
//! performance of various forms of the raz
//! data structures defined in this crate

extern crate rand;
extern crate time;
#[macro_use] extern crate clap;
extern crate stats;
extern crate adapton;
extern crate pmfp_collections;
extern crate eval;

use std::fs::OpenOptions;
use std::io::Write;
use rand::{StdRng,SeedableRng};
#[allow(unused)] use eval::types::*;
use eval::actions::*;
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
use eval::test_build::{TestResult,BuildTest};
use adapton::engine::manage::*;

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_UNITSIZE: usize = 1000;
const DEFAULT_NAMESIZE: usize = 1;
const DEFAULT_COUNT: usize = 15;
const DEFAULT_EDITS: usize = 1;
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
      -g, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -c, --count=[count]       'number of runs, by default each 2 is 10x larger'
      -e, --edits=[edits]       'number of edits'
      -t, --trials=[trials]     'trials to average over'
      -o, --outfile=[outfile]   'name for output files (of different extensions)' ")
    .get_matches();
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
	let unitsize = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namesize = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
  let count = value_t!(args, "count", usize).unwrap_or(DEFAULT_COUNT);
  let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);
  let outfile = args.value_of("outfile");

  let multiplier = f32::sqrt(10.0);

  //setup test
  let mut test = BuildTest{
    init: IncrementalInit {
      size: 0, // set in test
      datagauge: unitsize,
      namegauge: namesize,
      coord: StdRng::from_seed(&[dataseed]),
    },
    edit: BatchInsert(edits),
    multiplier: multiplier,
    count: count,
  };

  let _ = init_dcg(); assert!(engine_is_dcg());

  // run experiments
  let mut rng = StdRng::from_seed(&[editseed]);
  let result_raz: TestResult<EvalNRaz<usize,StdRng>> = test.test(&mut rng);
  let result_iraz: TestResult<EvalIRaz<usize,StdRng>> = test.test(&mut rng);
  let result_vec: TestResult<EvalVec<usize,StdRng>> = test.test(&mut rng);

  // post-process results
  let build_raz = result_raz.build.iter().map(|d|d.num_nanoseconds().unwrap());
  let build_iraz = result_iraz.build.iter().map(|d|d.num_nanoseconds().unwrap());
  let build_vec = result_vec.build.iter().map(|d|d.num_nanoseconds().unwrap());
  let build_all: Vec<((i64,i64),i64)> = build_raz.zip(build_iraz).zip(build_vec).collect();
  let edit_raz = result_raz.edit.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_iraz = result_iraz.edit.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_vec = result_vec.edit.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_all: Vec<((i64,i64),i64)> = edit_raz.zip(edit_iraz).zip(edit_vec).collect();
  
  println!("build time(raz,iraz,vec): {:?}", build_all);
  println!("edit time(raz,iraz,vec): {:?}", edit_all);

  ///////
  // Draft of output generation
  ///////

  let filename = if let Some(f) = outfile {f} else {"out"};

  let mut dat: Box<Write> =
    Box::new(
      OpenOptions::new()
      .create(true)
      .write(true)
      // truncate or append
      //.append(true)
      .truncate(true)
      .open(filename.to_owned()+".dat")
      .unwrap()
    )
  ;

  // generate data file
  writeln!(dat,"'{}'\t'{}'\t'{}'","Size","Build Time","Edit Time").unwrap();
  for i in 0..count {
    let size = multiplier.powi(i as i32) as usize;
    let br = (build_all[i].0).0 as f64 / 1_000_000.0;
    let ed = (edit_all[i].0).0 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}",size,br,ed).unwrap();
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..count {
    let size = multiplier.powi(i as i32) as usize;
    let br = (build_all[i].0).1 as f64 / 1_000_000.0;
    let ed = (edit_all[i].0).1 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}",size,br,ed).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..count {
    let size = multiplier.powi(i as i32) as usize;
    let bv = build_all[i].1 as f64 / 1_000_000.0;
    let ed = edit_all[i].1 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}",size,bv,ed).unwrap();    
  }

  let mut plotscript =
    OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filename.to_owned()+".plotscript")
    .unwrap()
  ;

  writeln!(plotscript,"set terminal pdf").unwrap();
  writeln!(plotscript,"set logscale xy").unwrap();
  writeln!(plotscript,"set output '{}'", filename.to_owned()+".pdf").unwrap();
  write!(plotscript,"set title \"{}", "Time to Build and Edit a Sequence\\n").unwrap();
  writeln!(plotscript,"(g)auge: {}, (n)ame-gauge: {}\"",unitsize,namesize).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "size").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat","NonIncRaz build time").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat","IncRaz build time").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat","Vec build time").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3 t '{}' with lines,\\",filename.to_owned()+".dat","NonIncRaz edit time").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with lines,\\",filename.to_owned()+".dat","IncRaz edit time").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:3 t '{}' with lines,\\",filename.to_owned()+".dat","Vec edit time").unwrap();

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
