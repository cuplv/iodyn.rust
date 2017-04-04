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
extern crate adapton_lab;

use std::io::BufWriter;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use rand::{StdRng,SeedableRng};
use eval::actions::*;
use eval::types::*;
use adapton_lab::labviz::*;
#[allow(unused)] use eval::types::*;
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::accum_lists::*;
use pmfp_collections::inc_gauged_trie::{FinMap,Trie};
use eval::test_seq::{TestMResult,EditComputeSequence};
use adapton::engine::manage::*;
use adapton::engine::*;
use eval::interface::*;

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
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -t, --trials=[trials]     'trials to average over'
      -o, --outfile=[outfile]   'name for output files (of different extensions)'
      --trace                   'produce an output trace of the incremental run' ")
    .get_matches();
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
	let start_size = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let unitgauge = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namegauge = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace");
  let coord = StdRng::from_seed(&[dataseed]);

  // Fold over raz, use vec as set to gather elements
  let mut testtree = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
      name_of_string(String::from("fillvec")),
      Vec::new(),
      |mut v,&GenSmall(e)|{v.push(e.clone());v.sort();v.dedup();v},
      |v,_|{v},
      |a|{a},
    ),
    changes: changes,
  };
  //fold over raz, use trie as set to gather elements
  let mut testtrie = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
      name_of_string(String::from("filltrie")),
      Trie::emp(),
      |mut a,&GenSmall(e)|{ a.put(e, ()); a },
      |mut a,(_lev,_nmopt)|{ match _nmopt {None => {a}, Some(nm) => {a.archive(nm);a} }},
      |a|{a},
    ),
    changes: changes,
  };
  // fold over raz, use vec to store all elements
  let mut test_vl = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
      name_of_string(String::from("to_list")),
      VecList::new(),
      |a,e|{a.seq_push(*e)},
      |a,(m,n)|{a.archive((m,n))},
      |a|{a},
    ),
    changes: changes,
  };

  let _ = init_dcg(); assert!(engine_is_dcg());

  // run experiments

  let mut rng = StdRng::from_seed(&[editseed]);
  let result_raz: TestMResult<
    EvalIRaz<GenSmall,StdRng>,
    Vec<usize>,
  > = testtree.test(&mut rng);

  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin()}

  let result_trie: TestMResult<
    EvalIRaz<GenSmall,StdRng>,
    Trie<usize,()>,
  > = testtrie.test(&mut rng);

  if do_trace {
    let traces = reflect::dcg_reflect_end();

    // output trace
    let f = File::create("trace.html").unwrap();
    let mut writer = BufWriter::new(f);
    writeln!(writer, "{}", style_string()).unwrap();
    writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
    writeln!(writer, "<div class=\"traces\">").unwrap();
    for tr in traces {
      div_of_trace(&tr).write_html(&mut writer);
    }
  }

  let inc_veclist: TestMResult<
    EvalIRaz<GenSmall,StdRng>,
    VecList<GenSmall>,
  > = ns(name_of_string(String::from("veclist")),||{test_vl.test(&mut rng)});


  // post-process results
  let comp_vec = result_raz.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_trie = result_trie.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_ivl = inc_veclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_vec = result_raz.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_trie = result_trie.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_ivl = inc_veclist.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  

  println!("Computation time(ns): (initial run, first incremental run)");
  println!("vec_set: ({:?}, {:?})", comp_vec[0], comp_vec[1]);
  println!("trie_set: ({:?}, {:?})", comp_trie[0], comp_trie[1]);
  println!("vec_list: ({:?}, {:?})", comp_ivl[0], comp_ivl[1]);

  ///////
  // Output generation
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
  let (mut e,mut c);
  writeln!(dat,"'{}'\t'{}'\t'{}'\t'{}'","Changes","Edit Time","Compute Time","Edit and Compute").unwrap();
  e = 0.0; c = 0.0;
  for i in 0..changes {
    e += edit_vec[i] as f64 / 1_000_000.0;
    c += comp_vec[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,e,c,e+c).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  e = 0.0; c = 0.0;
  for i in 0..changes {
    e += edit_trie[i] as f64 / 1_000_000.0;
    c += comp_trie[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,e,c,e+c).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  e = 0.0; c = 0.0;
  for i in 0..changes {
    e += edit_ivl[i] as f64 / 1_000_000.0;
    c += comp_ivl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,e,c,e+c).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();


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
  write!(plotscript,"set title \"{}", "Accumulating time to insert element(s) and build a set/list\\n").unwrap();
  writeln!(plotscript,"(s)ize: {}, (u)nit-gauge: {}, (n)ame-gauge: {}, (e)dit-batch: {}\"", start_size,unitgauge,namegauge,edits).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "(c)hanges").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"set grid ytics mytics  # draw lines for each ytics and mytics").unwrap();
  writeln!(plotscript,"set grid xtics mxtics  # draw lines for each xtics and mxtics").unwrap();
  writeln!(plotscript,"set mytics 5           # set the spacing for the mytics").unwrap();
  writeln!(plotscript,"set mxtics 5           # set the spacing for the mxtics").unwrap();
  writeln!(plotscript,"set grid               # enable the grid").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Vec edit").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Vec compute").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Vec total").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Trie edit").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Trie compute").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Trie total").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "List edit").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","List compute").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","List total").unwrap();

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
