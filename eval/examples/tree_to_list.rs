//! A test of incremental fold and output data structure types.
//!
//! Here we test the incremental fold over the RazTree, as
//! well as its interaction with multiple accumulator types.
//! 
//! The test generates a random, probabilistically balanced tree
//! of integers. We then perform an incremental fold over this
//! tree, passing some type of sequence as an accumulator. The
//! built sequence contains a copy of each element from the tree.
//! The point is to test the performance of breaking the sequence
//! into short arrays, or using sharable sequences vs owned ones.
//! 
//! Some of the tests are run with the incremental engine turned
//! off, which avoids most of the overhead and benefits it usually
//! provides.
//! 
//! Output sequence types:
//! - Stack: strict naming structure with arrays
//! - List: loose naming structure with no arrays
//! - RefList: loose naming structure with no arrays and reference-counted pointers
//! - VecList: loose naming structure with arrays
//! - RefVecList: loose naming structure with arrays and reference-counted pointers
//! - Vec: No names or pointers, one single array, generated from an array rather than the RazTree
//! 

extern crate rand;
#[macro_use] extern crate clap;
extern crate adapton;
extern crate adapton_lab;
extern crate iodyn;
extern crate eval;

use std::io::BufWriter;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use rand::{StdRng,SeedableRng};
use adapton::engine::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
#[allow(unused)] use iodyn::{IRaz,IRazTree};
#[allow(unused)] use iodyn::inc_archive_stack::AStack as IAStack;
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::eval_iastack::EvalIAStack;
#[allow(unused)] use eval::examples::*;
#[allow(unused)] use eval::accum_lists::*;
use eval::test_seq::{TestMResult,EditComputeSequence};
use eval::actions::*;
use eval::interface::*;
use eval::types::*;

fn main () {

// provide additional stack memory
// TODO: make macro?
  let child =
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}
fn main2() {
// end provide additional stack memory

  //command-line
  let args = clap::App::new("tree_to_list")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Simple conversion test")
    .args_from_usage("\
      --dataseed=[dataseed]			  'seed for random data'
      --editseed=[edit_seed]      'seed for random edits (and misc.)'
      -s, --start=[start]         'starting sequence length'
      -g, --datagauge=[datagauge] 'initial elements per structure unit'
      -n, --namesize=[namesize]   'initial tree nodes between each art'
      -e, --edits=[edits]         'edits per batch'
      -c, --changes=[changes]     'number of incremental changes'
      -o, --outfile=[outfile]     'name for output files (of different extensions)'
      --trace                     'perform dcg debugging trace of stack output' ")
    .get_matches();
  let dataseed = value_t!(args, "data_seed", usize).unwrap_or(0);
  let editseed = value_t!(args, "edit_seed", usize).unwrap_or(0);
	let start_size = value_t!(args, "start", usize).unwrap_or(1_000_000);
	let datagauge = value_t!(args, "datagauge", usize).unwrap_or(1_000);
	let namegauge = value_t!(args, "namesize", usize).unwrap_or(1);
	let edits = value_t!(args, "edits", usize).unwrap_or(1);
	let changes = value_t!(args, "changes", usize).unwrap_or(30);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace");
	let coord = StdRng::from_seed(&[dataseed]);

	// add elements to output sequence
	fn to_list_step<E:Copy, A: IFaceSeq<E>>(a:A,e:&E) -> A {
		a.seq_push(*e)
	}
	// add names to output sequence 
	fn to_list_meta<M, A: IFaceArchive<(M,Option<Name>)>>(a:A,(m,n):(M,Option<Name>)) -> A {
		a.archive((m,n))
	}

	// The test parameters, copied multiple times because
	// taking polymorphic functions as parameters requires a
	// new data structure for each set of function type parameters
  let mut test_s = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
  		name_of_string(String::from("to_list")),
			IFaceNew::new(),
			to_list_step,
			to_list_meta,
			|a|{a},
		),
    changes: changes,
  };
  let mut test_l = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
  		name_of_string(String::from("to_list")),
			IFaceNew::new(),
			to_list_step,
			to_list_meta,
			|a|{a},
		),
    changes: changes,
  };
  let mut test_rl = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
  		name_of_string(String::from("to_list")),
			IFaceNew::new(),
			to_list_step,
			to_list_meta,
			|a|{a},
		),
    changes: changes,
  };
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
			IFaceNew::new(),
			to_list_step,
			to_list_meta,
			|a|{a},
		),
    changes: changes,
  };
  let mut test_rvl = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
  		name_of_string(String::from("to_list")),
			IFaceNew::new(),
			to_list_step,
			to_list_meta,
			|a|{a},
		),
    changes: changes,
  };
  let mut test_v = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: MFolder::new(
  		name_of_string(String::from("to_list")),
			IFaceNew::new(),
			to_list_step,
			to_list_meta::<(),_>,
			|a|{a},
		),
    changes: changes,
  };

  
  // run experiments


  let mut rng = StdRng::from_seed(&[editseed]);

  let noninc_stack: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	IAStack<GenSmall,()>,
  > = test_s.test(&mut rng); 

  let noninc_list: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	List<GenSmall>,
  > = test_l.test(&mut rng); 

  let noninc_rclist: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	RcList<GenSmall>,
  > = test_rl.test(&mut rng); 

  let noninc_veclist: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	VecList<GenSmall>,
  > = test_vl.test(&mut rng); 

  let noninc_rcveclist: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	RcVecList<GenSmall>,
  > = test_rvl.test(&mut rng); 

  let noninc_vec: TestMResult<
  	EvalVec<GenSmall,StdRng>,
  	Vec<GenSmall>,
  > = test_v.test(&mut rng);

  // starting now all tests are incremental
	init_dcg(); assert!(engine_is_dcg());

  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin();}

  let inc_stack: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	IAStack<GenSmall,()>,
  > = ns(name_of_string(String::from("stack")),||{test_s.test(&mut rng)});

  if do_trace {
	  // Generate trace of inc stack
		let traces = reflect::dcg_reflect_end();
	  let f = File::create("trace.html").unwrap();
	  let mut writer = BufWriter::new(f);
	  writeln!(writer, "{}", style_string()).unwrap();
	  writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
	  writeln!(writer, "<div class=\"traces\">").unwrap();
	  for tr in traces {
	  	div_of_trace(&tr).write_html(&mut writer);
	  }
	}

  let inc_list: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	List<GenSmall>,
  > = ns(name_of_string(String::from("list")),||{test_l.test(&mut rng)});

  let inc_rclist: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	RcList<GenSmall>,
  > = ns(name_of_string(String::from("rclist")),||{test_rl.test(&mut rng)});

  let inc_veclist: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	VecList<GenSmall>,
  > = ns(name_of_string(String::from("veclist")),||{test_vl.test(&mut rng)});

  let inc_rcveclist: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	RcVecList<GenSmall>,
  > = ns(name_of_string(String::from("rcveclist")),||{test_rvl.test(&mut rng)});

  
  // post-process results


  let comp_ns = noninc_stack.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_nl = noninc_list.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_nrl = noninc_rclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_nvl = noninc_veclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_nrvl = noninc_rcveclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_nv = noninc_vec.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_is = inc_stack.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_il = inc_list.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_irl = inc_rclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_ivl = inc_veclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_irvl = inc_rcveclist.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  
  println!("Computation time(ns): (initial run, first incremental run)");
  println!("noninc_stack: ({:?}, {:?})", comp_ns[0], comp_ns[1]);
  println!("noninc_list: ({:?}, {:?})", comp_nl[0], comp_nl[1]);
  println!("noninc_rclist: ({:?}, {:?})", comp_nrl[0], comp_nrl[1]);
  println!("noninc_veclist: ({:?}, {:?})", comp_nvl[0], comp_nvl[1]);
  println!("noninc_rcveclist: ({:?}, {:?})", comp_nrvl[0], comp_nrvl[1]);
  println!("noninc_vec: ({:?}, {:?})", comp_nv[0], comp_nv[1]);
  println!("inc_stack: ({:?}, {:?})", comp_is[0], comp_is[1]);
  println!("inc_list: ({:?}, {:?})", comp_il[0], comp_il[1]);
  println!("inc_rclist: ({:?}, {:?})", comp_irl[0], comp_irl[1]);
  println!("inc_veclist: ({:?}, {:?})", comp_ivl[0], comp_ivl[1]);
  println!("inc_rcveclist: ({:?}, {:?})", comp_irvl[0], comp_irvl[1]);


  // generate data file


  let filename = if let Some(f) = outfile {f} else {"out"};
  println!("Generating {}.pdf ...", filename);

  let mut dat: Box<Write> =
    Box::new(
      OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open(filename.to_owned()+".dat")
      .unwrap()
    )
  ;

  let (mut nl,mut nrl,mut nvl,mut nrvl,mut ns,mut il,mut irl,mut ivl,mut irvl,mut is,mut nv) = (0f64,0f64,0f64,0f64,0f64,0f64,0f64,0f64,0f64,0f64,0f64);
  writeln!(dat,"'{}'\t'{}'","Trial","Compute Time").unwrap();
  for i in 0..changes {
    ns += comp_ns[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,ns).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    nl += comp_nl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,nl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    nrl += comp_nrl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,nrl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    nvl += comp_nvl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,nvl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    nrvl += comp_nrvl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,nrvl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    nv += comp_nv[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,nv).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    is += comp_is[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,is).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    il += comp_il[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,il).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    irl += comp_irl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,irl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    ivl += comp_ivl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,ivl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    irvl += comp_irvl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,irvl).unwrap();    
  }

  let mut plotscript =
    OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filename.to_owned()+".plotscript")
    .unwrap()
  ;

  // generate plot script

  writeln!(plotscript,"set terminal pdf").unwrap();
  writeln!(plotscript,"set output '{}'", filename.to_owned()+".pdf").unwrap();
  write!(plotscript,"set title \"{}", "Cumulative time to insert element(s) and build list/stack\\n").unwrap();
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
  let mut i = -1;
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-inc Stack").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i," Non-inc List").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-inc RcList").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-inc VecList").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i," Non-inc RcVecList").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Common Vec").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc Stack").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc List").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc RcList").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc VecList").unwrap();
  i+=1; writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc RcVecList").unwrap();

  //generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
