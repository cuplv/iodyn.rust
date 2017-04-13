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
use std::collections::HashMap;
use rand::{StdRng,SeedableRng};
use eval::actions::*;
use eval::types::*;
use adapton_lab::labviz::*;
#[allow(unused)] use eval::types::*;
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::accum_lists::*;
//use pmfp_collections::inc_gauged_trie::{FinMap,Trie};
use pmfp_collections::inc_gauged_trie_opt3::{FinMap,Skiplist};
use eval::test_seq::{TestMResult,EditComputeSequence};
use adapton::engine::manage::*;
use adapton::engine::*;
use adapton::engine::reflect::trace::*;
use eval::interface::*;

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_START: usize = 10_000;
const DEFAULT_UNITSIZE: usize = 100;
const DEFAULT_NAMESIZE: usize = 1;
const DEFAULT_EDITS: usize = 1;
const DEFAULT_CHANGES: usize = 30;
const DEFAULT_TRIALS: usize = 10;
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
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -t, --trials=[trials]     'trials to average over'
      -o, --outfile=[outfile]   'name for output files (of different extensions)'
      -p, --pathlen=[pathlen]   'length of paths in the skiplist'
      --trace                   'trace the incremental run, producing counts'
      --trace-html              'trace the incremental run, producing counts and HTML (can be very large)'")
    .get_matches();
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
	let start_size = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let unitgauge = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namegauge = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);
  let pathlen = value_t!(args, "pathlen", usize).unwrap_or(DEFAULT_PATHLEN);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace") || args.is_present("trace-html");
  let do_trace_html = args.is_present("trace-html");
  let coord = StdRng::from_seed(&[dataseed]);

  //fold over raz, use trie as set to gather elements
  let mut testtrie = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: HFolder::new(
      name_of_string(String::from("filltrie")),
      {let mut t = Skiplist::emp(pathlen, name_unit()); t.archive(name_unit()); t},
      |mut a,&GenSmall(e)|{ a.put(e, ()); a },
      |mut a,nm|{ match nm { None => a, Some(nm) => { a.archive(nm); a }}},
      |a,(_lev,_nmopt)|{ a },
      |a|{a},
    ),
    changes: changes,
  };

  let mut test_hashmap = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::<_,_,(),_,_,_,_,_>::new(
      name_of_string(String::from("fillhashmap")),
      HashMap::new(),
      |mut m,&GenSmall(e)|{m.insert(e,());m},
      |m,_|{m},
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
      |a,&GenSmall(e)|{a.seq_push(e)},
      |a,(m,n)|{a.archive((m,n))},
      |a|{a},
    ),
    changes: changes,
  };

  let _ = init_dcg(); assert!(engine_is_dcg());
  let mut rng = StdRng::from_seed(&[editseed]);

  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin()}

  // run experiments
    
  #[derive(Debug)]
  struct TraceCount {
      reeval_nochange: usize,
      reeval_change:   usize,
      alloc_fresh:     usize,
      alloc_nochange:  usize,
      alloc_change:    usize,
      dirty:           usize,
      clean_rec:       usize,
  }

  fn count_dirty(tr:&Trace, c:&mut TraceCount) -> usize {
      let dirty0 = c.dirty;
      match tr.effect {
          Effect::Dirty => c.dirty += 1,
          _ => (),
      };
      for sub_tr in tr.extent.iter() {
          count_dirty(sub_tr, c);
      };
      return c.dirty - dirty0;
  }

  fn count_clean(tr:&Trace, c:&mut TraceCount) {
      match tr.effect {
          Effect::CleanRec => c.clean_rec += 1,
          _ => (),
      };
      for sub_tr in tr.extent.iter() {
          count_clean(sub_tr, c);
      };
  }

  fn count_alloc_change(tr:&Trace, c:&mut TraceCount) -> usize {
      let alloc_change0 = c.alloc_change;
      match tr.effect {
          Effect::Alloc(AllocCase::LocFresh, _)                           => c.alloc_fresh    += 1,
          Effect::Alloc(AllocCase::LocExists(ChangeFlag::ContentDiff), _) => c.alloc_change   += 1,
          Effect::Alloc(AllocCase::LocExists(ChangeFlag::ContentSame), _) => c.alloc_nochange += 1,
          _ => (),
      };
      for sub_tr in tr.extent.iter() {
          count_alloc_change(sub_tr, c);
      };
      return c.alloc_change - alloc_change0;
  }

  fn count_reeval(tr:&Trace, c:&mut TraceCount) {
      match tr.effect {          
          reflect::trace::Effect::CleanEval => { 
              let dirty0 = c.dirty;
              if  count_dirty(tr, c) > 0 || 
                  count_alloc_change(tr, c) > 0 
                   { c.reeval_change   += 1 }
              else { c.reeval_nochange += 1 };
              c.dirty = dirty0;
          },
          _ => ()
      };
      for sub_tr in tr.extent.iter() { count_reeval(sub_tr, c) };
  }

  let result_trie: TestMResult<
    EvalIRaz<GenSmall,StdRng>,
    Skiplist<usize,()>,
  > = testtrie.test(&mut rng);

  if do_trace {
    let traces = reflect::dcg_reflect_end();
    
    // output analytic counts
    let mut count = TraceCount{ alloc_fresh:0, alloc_change:0, alloc_nochange:0,
                                reeval_change:0, reeval_nochange:0, 
                                dirty:0, clean_rec:0};
    for tr in &traces {
      count_reeval(&tr, &mut count);
      count_clean(&tr, &mut count);
      count_dirty(&tr, &mut count);
    };
    println!("{:?}", count);

    // output trace
    if do_trace_html {
    let f = File::create("trace.html").unwrap();
    let mut writer = BufWriter::new(f);
    writeln!(writer, "{}", style_string()).unwrap();
    writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
    writeln!(writer, "<div class=\"traces\">").unwrap();
    for tr in traces {
      div_of_trace(&tr).write_html(&mut writer);
    };
    }
  }

  let result_hash: TestMResult<
    EvalVec<GenSmall,StdRng>,
    HashMap<usize,()>,
  > = ns(name_of_string(String::from("hashmap")),||{test_hashmap.test(&mut rng)});

  let inc_veclist: TestMResult<
    EvalIRaz<GenSmall,StdRng>,
    VecList<usize>,
  > = ns(name_of_string(String::from("veclist")),||{test_vl.test(&mut rng)});

  // post-process results
  let comp_hash = result_hash.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_trie = result_trie.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_ivl = inc_veclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_hash = result_hash.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_trie = result_trie.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_ivl = inc_veclist.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  

  println!("Computation time (ms): (initial run, first incremental run); do_trace={:?}", do_trace);
  println!("hash_set: ({:8.3}, {:8.3})", comp_hash[0] as f32 / 1000000.0, comp_hash[1] as f32 / 1000000.0);
  println!("trie_set: ({:8.3}, {:8.3})", comp_trie[0] as f32 / 1000000.0, comp_trie[1] as f32 / 1000000.0);
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
  writeln!(plotscript,"'{}' i 0 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Non-Inc HashMap edit").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Non-Inc HashMap compute").unwrap();
  writeln!(plotscript,"'{}' i 0 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Non-Inc HashMap total").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Inc Skiplist edit").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Skiplist compute").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc Skiplist total").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:3:4 t '{}' with filledcu fs solid 0.1,\\",filename.to_owned()+".dat", "Inc List (store all) edit").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc List (store all) compute").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:4 t '{}' with linespoints,\\",filename.to_owned()+".dat","Inc List (store all) total").unwrap();

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
