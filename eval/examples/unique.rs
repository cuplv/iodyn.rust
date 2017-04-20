//! An old version of our test to find unique elements in a list

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
use eval::actions::*;
use eval::types::*;
#[allow(unused)] use eval::types::*;
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
use eval::test_seq::{TestResult,EditComputeSequence};
use adapton::engine::manage::*;

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_START: usize = 10_000;
const DEFAULT_UNITSIZE: usize = 100;
const DEFAULT_NAMESIZE: usize = 1;
const DEFAULT_EDITS: usize = 1;
const DEFAULT_CHANGES: usize = 30;
const DEFAULT_TRIALS: usize = 10;

fn main () {
  use std::thread;
  let child =
    thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}
fn main2() {
  //command-line
  let args = clap::App::new("chartraz")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Produces comparison charts for RAZ data structure")
    .args_from_usage("\
      --dataseed=[dataseed]			'seed for random data'
      --editseed=[edit_seed]    'seed for random edits (and misc.)'
      -s, --start=[start]       'starting sequence length'
      -g, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -t, --trials=[trials]     'trials to average over'
      -o, --outfile=[outfile]   'name for output files (of different extensions)' ")
    .get_matches();
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
	let start = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let unitsize = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namesize = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);
  let outfile = args.value_of("outfile");

  let mut testtree = EditComputeSequence{
    init: IncrementalInit {
      size: start,
      datagauge: unitsize,
      namegauge: namesize,
      coord: StdRng::from_seed(&[dataseed]),
    },
    edit: BatchInsert(edits),
    comp: TreeFold::new(|&Gen10k(d)|{
      vec![d]
    },|m,n|{
      let (mut m,n) = (m,n);
      m.extend(n);
      m.sort();
      m.dedup();
      m
    }),
    changes: changes,
  };
  let mut testlr = EditComputeSequence{
    init: IncrementalInit {
      size: start,
      datagauge: unitsize,
      namegauge: namesize,
      coord: StdRng::from_seed(&[dataseed]),
    },
    edit: BatchInsert(edits),
    comp: Folder::new(vec![],|m,&Gen10k(n)|{
      let mut m = m;
      m.push(n);
      m.sort();
      m.dedup();
      m
    }),
    changes: changes,
  };

  let _ = init_dcg(); assert!(engine_is_dcg());

  // run experiments
  let mut rng = StdRng::from_seed(&[editseed]);
  let result_raz: TestResult<EvalIRaz<Gen10k,StdRng>> = testtree.test(&mut rng);
  let result_vec: TestResult<EvalVec<Gen10k,StdRng>> = testlr.test(&mut rng);

  // post-process results
  let comp_raz = result_raz.computes.iter().map(|d|d.num_nanoseconds().unwrap());
  let comp_vec = result_vec.computes.iter().map(|d|d.num_nanoseconds().unwrap());
  let comp_both: Vec<(i64,i64)> = comp_raz.zip(comp_vec).collect();
  let edit_raz = result_raz.edits.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_vec = result_vec.edits.iter().map(|d|d.num_nanoseconds().unwrap());
  let edit_both: Vec<(i64,i64)> = edit_raz.zip(edit_vec).collect();
  
  println!("edits(raz,vec): {:?}", edit_both);
  println!("computes(raz,vec): {:?}", comp_both);
  //println!("answers: {:?}", answers);

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
  let (mut er,mut ev,mut cr,mut cv) = (0f64,0f64,0f64,0f64);
  writeln!(dat,"'{}'\t'{}'\t'{}'\t'{}'","Trial","Edit Time","Compute Time","Edit and Compute").unwrap();
  for i in 0..changes {
    er += edit_both[i].0 as f64 / 1_000_000.0;
    cr += comp_both[i].0 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,er,cr,er+cr).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    ev += edit_both[i].1 as f64 / 1_000_000.0;
    cv += comp_both[i].1 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,ev,cv,ev+cv).unwrap();    
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
  writeln!(plotscript,"set output '{}'", filename.to_owned()+".pdf").unwrap();
  write!(plotscript,"set title \"{}", "Accumulating time to insert element(s) and build a set\\n").unwrap();
  writeln!(plotscript,"(s)ize: {}, (g)auge: {}, (n)ame-gauge: {}, (e)dit-batch: {}\"", start,unitsize,namesize,edits).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "(c)hanges").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Raz edit").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3 t '{}' with lines,\\",filename.to_owned()+".dat","Raz compute").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:4 t '{}' with lines,\\",filename.to_owned()+".dat","Raz total").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Vec edit").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with lines,\\",filename.to_owned()+".dat","Vec compute").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:4 t '{}' with lines,\\",filename.to_owned()+".dat","Vec total").unwrap();

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
