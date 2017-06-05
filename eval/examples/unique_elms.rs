//! This is a system for creating charts of the
//! performance of various forms of the raz
//! data structures defined in this crate

extern crate rand;
extern crate time;
#[macro_use] extern crate clap;
//extern crate stats;
extern crate adapton;
extern crate iodyn;
extern crate eval;
extern crate adapton_lab;

//use std::fmt;
use std::io::BufWriter;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;
use rand::{StdRng,SeedableRng};
use eval::actions::*;
use eval::types::*;
use adapton::reflect;
use adapton_lab::labviz::*;
#[allow(unused)] use eval::types::*;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::accum_lists::*;

//use iodyn::inc_gauged_trie::{FinMap,Trie};
//use iodyn::skiplist::{FinMap,Skiplist};
type Trie1<K,V> = iodyn::trie1::Trie<K,V>;
type Trie2<K,V> = iodyn::trie2::Trie<K,V>;

use eval::test_seq::{TestResult,TestMResult,EditComputeSequence};
use adapton::engine::manage::*;
use adapton::engine::*;
use adapton::reflect::trace::*;
use eval::interface::*;

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_START: usize = 100_000;
const DEFAULT_UNITSIZE: usize = 1000;
const DEFAULT_NAMESIZE: usize = 1;
const DEFAULT_EDITS: usize = 1;
const DEFAULT_CHANGES: usize = 30;
//const DEFAULT_TRIALS: usize = 10;
const DEFAULT_PATHLEN: usize = 32;

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
      -o, --outfile=[outfile]   'name for output files (of different extensions)'
      -p, --pathlen=[pathlen]   'length of paths in the skiplist'
      --trace                   'trace the incremental run, producing counts'
      --trace-html              'trace the incremental run, producing counts and HTML (can be very large)'")
    .get_matches();
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
	let start_size = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let datagauge = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namegauge = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
//  let pathlen = value_t!(args, "pathlen", usize).unwrap_or(DEFAULT_PATHLEN);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace") || args.is_present("trace-html");
  let do_trace_html = args.is_present("trace-html");
  let coord = StdRng::from_seed(&[dataseed]);

  let mut test_hashmap = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::<_,_,(),_,_,_,_,_>::new(
      name_of_string(String::from("fillhashmap")),
      HashMap::new(),
      |mut m,&GenSetElm(e)|{m.insert(e,());m},
      |m,_|{m},
      |a|{a},
    ),
    changes: changes,
  };


  let mut test_trie1 = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: TreeFoldG::new(
      |v:&Vec<GenSetElm>|{ Trie1::<_,()>::from_key_vec_ref(v) },
      move|t1,_lev,nm,t2|{ 
          let nm2 = nm.clone();
          ns(nm.unwrap().clone(), || Trie1::join(nm2,t1,t2) )
      },
    ),
    changes: changes,
  };

  let mut test_trie2 = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: TreeFoldG::new(
      |v:&Vec<GenSetElm>|{ Trie2::<_,()>::from_key_vec_ref(v) },
      move|t1,_lev,nm,t2|{ 
          let nm2 = nm.clone();
          ns(nm.unwrap().clone(), || Trie2::join(Some(nm2.unwrap()),t1,t2) )
      },
    ),
    changes: changes,
  };

  // fold over raz, use vec to store all elements
  let mut test_vl = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
      name_of_string(String::from("to_list")),
      VecList::new(),
      |a,&GenSetElm(e)|{a.seq_push(e)},
      |a,(m,n)|{a.archive((m,n))},
      |a|{a},
    ),
    changes: changes,
  };

  // Warm up the Rust process in the OS:

  let _ = init_dcg(); assert!(engine_is_dcg());
  let mut rng = StdRng::from_seed(&[editseed]);
  let result_hash_warmup: TestMResult<
    EvalVec<GenSetElm,StdRng>,
    HashMap<Elm,()>,
  > = ns(name_of_string(String::from("hashmap")),||{test_hashmap.test(&mut rng)});
  drop(result_hash_warmup);
    
  // Measure tests, post warm-up test.

  let _ = init_dcg(); assert!(engine_is_dcg());
  let mut rng = StdRng::from_seed(&[editseed]);
  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin()}

  //let result_trie1: TestResult<EvalIRaz<GenSetElm,StdRng>,_> = test_trie1.test(&mut rng);

  if do_trace {
    let traces = reflect::dcg_reflect_end();
    
    // output analytic counts
    let count = trace_count(&traces, Some(changes));
    println!("{:?}", count);

    // output trace
    if do_trace_html {
    let f = File::create("trace_trie1.html").unwrap();
    let mut writer = BufWriter::new(f);
    writeln!(writer, "{}", style_string()).unwrap();
    writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
    writeln!(writer, "<div class=\"traces\">").unwrap();
    for tr in traces {
      div_of_trace(&tr).write_html(&mut writer);
    };
    }
  }

  let _ = init_dcg(); assert!(engine_is_dcg());
  let mut rng = StdRng::from_seed(&[editseed]);
  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin()}

  let result_trie2: TestResult<EvalIRaz<GenSetElm,StdRng>,_> = test_trie2.test(&mut rng);

  if do_trace {
    let traces = reflect::dcg_reflect_end();
    
    // output analytic counts
    let count = trace_count(&traces, Some(changes));
    println!("{:?}", count);

    // output trace
    if do_trace_html {
    let f = File::create("trace_trie2.html").unwrap();
    let mut writer = BufWriter::new(f);
    writeln!(writer, "{}", style_string()).unwrap();
    writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
    writeln!(writer, "<div class=\"traces\">").unwrap();
    for tr in traces {
      div_of_trace(&tr).write_html(&mut writer);
    };
    }
  }

  let _ = init_dcg(); assert!(engine_is_dcg());
  let mut rng = StdRng::from_seed(&[editseed]);
  let result_hash: TestMResult<
    EvalVec<GenSetElm,StdRng>,
    HashMap<Elm,()>,
  > = ns(name_of_string(String::from("hashmap")),||{test_hashmap.test(&mut rng)});

  let _ = init_dcg(); assert!(engine_is_dcg());
  let mut rng = StdRng::from_seed(&[editseed]);
  let inc_veclist: TestMResult<
    EvalIRaz<GenSetElm,StdRng>,
    VecList<Elm>,
  > = ns(name_of_string(String::from("veclist")),||{test_vl.test(&mut rng)});

  // post-process results
  let comp_hash = result_hash.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  //let comp_trie1 = result_trie1.computes.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_trie2 = result_trie2.computes.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_ivl = inc_veclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();

  let edit_hash = result_hash.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  //let edit_trie1 = result_trie1.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_trie2 = result_trie2.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_ivl = inc_veclist.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  

  println!("----------------------------------------------------------------------------------");
  println!("Computation time (ms): (initial run, first incremental run); Note:do_trace={:?}", do_trace);
  println!("hashmap:  ({:8.3}, {:8.3})", comp_hash[0] as f32 / 1000000.0, comp_hash[1] as f32 / 1000000.0);
  //println!("trie1:     ({:8.3}, {:8.3})", comp_trie1[0] as f32 / 1000000.0, comp_trie1[1] as f32 / 1000000.0);
  println!("trie2:     ({:8.3}, {:8.3})", comp_trie2[0] as f32 / 1000000.0, comp_trie2[1] as f32 / 1000000.0);
  println!("vec_list: ({:8.3}, {:8.3})", comp_ivl[0]  as f32 / 1000000.0, comp_ivl[1]  as f32 / 1000000.0);

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
    e += edit_hash[i] as f64 / 1_000_000.0;
    c += comp_hash[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,e,c,e+c).unwrap();    
  }
  // writeln!(dat,"").unwrap();
  // writeln!(dat,"").unwrap();
  // e = 0.0; c = 0.0;
  // for i in 0..changes {
  //   e += edit_trie1[i] as f64 / 1_000_000.0;
  //   c += comp_trie1[i] as f64 / 1_000_000.0;
  //   writeln!(dat,"{}\t{}\t{}\t{}",i,e,c,e+c).unwrap();    
  // }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  e = 0.0; c = 0.0;
  for i in 0..changes {
    e += edit_trie2[i] as f64 / 1_000_000.0;
    c += comp_trie2[i] as f64 / 1_000_000.0;
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
  write!(plotscript,"set title \"{}", "Cumulative edit/compute times vs Number of edits/updates\\n").unwrap();
  writeln!(plotscript,"(s)ize: {}, (g)auge: {}, (n)ame-gauge: {}, (e)dit-batch: {}\"", start_size,datagauge,namegauge,edits).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "(c)hanges").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"set grid ytics mytics  # draw lines for each ytics and mytics").unwrap();
  writeln!(plotscript,"set grid xtics mxtics  # draw lines for each xtics and mxtics").unwrap();
  writeln!(plotscript,"set mytics 5           # set the spacing for the mytics").unwrap();
  writeln!(plotscript,"set mxtics 5           # set the spacing for the mxtics").unwrap();
  writeln!(plotscript,"set grid               # enable the grid").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  
  //writeln!(plotscript,"'{}' i 0 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Non-Inc HashMap edit").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","(Native) HashMap").unwrap();
  //writeln!(plotscript,"'{}' i 0 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Non-Inc HashMap total").unwrap();

  //writeln!(plotscript,"'{}' i 1 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Inc Skiplist edit").unwrap();
  //writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Skiplist").unwrap();
  //writeln!(plotscript,"'{}' i 1 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Skiplist total").unwrap();

  //writeln!(plotscript,"'{}' i 1 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Inc Trie edit").unwrap();
  //writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Trie1").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Trie2").unwrap();
  //writeln!(plotscript,"'{}' i 1 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Trie total").unwrap();

  //writeln!(plotscript,"'{}' i 2 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Inc List edit").unwrap();
  writeln!(plotscript,"'{}' i 3 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc List").unwrap();
  //writeln!(plotscript,"'{}' i 2 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc List total").unwrap();

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
