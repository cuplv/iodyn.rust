extern crate pmfp_collections;
extern crate adapton;
extern crate eval;
extern crate time;
extern crate rand;
#[macro_use] extern crate clap;

extern crate adapton_lab;

use rand::{StdRng,SeedableRng};
#[allow(unused)] use pmfp_collections::{IRaz,IRazTree};
#[allow(unused)] use pmfp_collections::inc_archive_stack::AStack as IAStack;
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
use adapton::engine::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
//use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

fn main () {

  let child =
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}
fn main2() {

  //command-line
  let args = clap::App::new("tree_to_list")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Simple conversion test")
    .args_from_usage("\
      --dataseed=[dataseed]			'seed for random data'
      --editseed=[edit_seed]    'seed for random edits (and misc.)'
      -s, --start=[start]       'starting sequence length'
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -o, --outfile=[outfile]   'name for output files (of different extensions)'
      --trace                   'perform dcg debugging trace of stack output' ")
    .get_matches();
  let dataseed = value_t!(args, "data_seed", usize).unwrap_or(0);
  let editseed = value_t!(args, "edit_seed", usize).unwrap_or(0);
	let start_size = value_t!(args, "start", usize).unwrap_or(1_000_000);
	let unitgauge = value_t!(args, "unitsize", usize).unwrap_or(1_000);
	let namegauge = value_t!(args, "namesize", usize).unwrap_or(1);
	let edits = value_t!(args, "edits", usize).unwrap_or(1);
	let changes = value_t!(args, "changes", usize).unwrap_or(30);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace");

	let coord = StdRng::from_seed(&[dataseed]);

	fn to_list_step<E:Copy, A: IFaceSeq<E>>(a:A,e:&E) -> A {
		a.seq_push(*e)
	}
	fn to_list_meta<M, A: IFaceArchive<(M,Option<Name>)>>(a:A,(m,n):(M,Option<Name>)) -> A {
		a.archive((m,n))
	}

  let mut test_s = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
    //init: IncrementalFrom {
    //	data: iraztree_depth_4(),

      unitgauge: unitgauge,
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
    //init: IncrementalFrom {
    //	data: iraztree_depth_4(),

      unitgauge: unitgauge,
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
    //init: IncrementalFrom {
    //	data: iraztree_depth_4(),

      unitgauge: unitgauge,
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
    //init: IncrementalFrom {
    //	data: iraztree_depth_4(),

      unitgauge: unitgauge,
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
    //init: IncrementalFrom {
    //	data: iraztree_depth_4(),

      unitgauge: unitgauge,
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
    //init: IncrementalFrom {
    //	data: iraztree_depth_4(),

      unitgauge: unitgauge,
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
  
  println!("computes(noninc_stack): {:?}", comp_ns);
  println!("computes(noninc_list): {:?}", comp_nl);
  println!("computes(noninc_rclist): {:?}", comp_nrl);
  println!("computes(noninc_veclist): {:?}", comp_nvl);
  println!("computes(noninc_rcveclist): {:?}", comp_nrvl);
  println!("computes(noninc_vec): {:?}", comp_nv);
  println!("computes(inc_stack): {:?}", comp_is);
  println!("computes(inc_list): {:?}", comp_il);
  println!("computes(inc_rclist): {:?}", comp_irl);
  println!("computes(inc_veclist): {:?}", comp_ivl);
  println!("computes(inc_rcveclist): {:?}", comp_irvl);


  // generate data file


  let filename = if let Some(f) = outfile {f} else {"out"};

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
  write!(plotscript,"set title \"{}", "Accumulating time to insert element(s) and build list/stack\\n").unwrap();
  writeln!(plotscript,"(s)ize: {}, (u)nit-gauge: {}, (n)ame-gauge: {}, (e)dit-batch: {}\"", start_size,unitgauge,namegauge,edits).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "(c)hanges").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  let mut i = 0;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-inc Stack").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i," Non-inc List").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-inc RcList").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-inc VecList").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i," Non-inc RcVecList").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Non-Inc Vec").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc Stack").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc List").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc RcList").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc VecList").unwrap(); i+=1;
  writeln!(plotscript,"'{}' i {} u 1:2 t '{}' with linespoints,\\",filename.to_owned()+".dat",i,"Inc RcVecList").unwrap(); i+=1;

  //generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
