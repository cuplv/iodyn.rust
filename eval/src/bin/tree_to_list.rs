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
      -o, --outfile=[outfile]   'name for output files (of different extensions)' ")
    .get_matches();
  let dataseed = value_t!(args, "data_seed", usize).unwrap_or(0);
  let editseed = value_t!(args, "edit_seed", usize).unwrap_or(0);
	let start_size = value_t!(args, "start", usize).unwrap_or(10_000);
	let unitgauge = value_t!(args, "unitsize", usize).unwrap_or(100);
	let namegauge = value_t!(args, "namesize", usize).unwrap_or(1);
	let edits = value_t!(args, "edits", usize).unwrap_or(1);
	let changes = value_t!(args, "changes", usize).unwrap_or(30);
  let outfile = args.value_of("outfile");

	let coord = StdRng::from_seed(&[dataseed]);

	fn to_list_step<E:Copy, A: IFaceSeq<E>>(a:A,e:&E) -> A {
		a.seq_push(*e)
	}
	fn to_list_meta<M, A: IFaceArchive<(M,Option<Name>)>>(a:A,(m,n):(M,Option<Name>)) -> A {
		a.archive((m,n))
	}

  let mut test = EditComputeSequence{
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

  
  // run experiments


  let mut rng = StdRng::from_seed(&[editseed]);

  let noninc_list: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	List<GenSmall>,
  > = test.test(&mut rng); 

  let noninc_stack: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	List<GenSmall>,
  > = test.test(&mut rng); 

  init_dcg(); assert!(engine_is_dcg());

  let inc_list: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	List<GenSmall>,
  > = test.test(&mut rng); 

  // for visual debugging
  reflect::dcg_reflect_begin();

  let inc_stack: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,
  	List<GenSmall>,
  > = test.test(&mut rng); 


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

  
  // post-process results


  let comp_nl = noninc_list.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_ns = noninc_stack.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_il = inc_list.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_is = inc_stack.computes.iter().map(|d|d[0].num_nanoseconds().unwrap()).collect::<Vec<_>>();
  
  println!("computes(noninc_list): {:?}", comp_nl);
  println!("computes(noninc_stack): {:?}", comp_ns);
  println!("computes(inc_list): {:?}", comp_il);
  println!("computes(inc_stack): {:?}", comp_is);


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

  let (mut nl,mut ns,mut il,mut is) = (0f64,0f64,0f64,0f64);
  writeln!(dat,"'{}'\t'{}'","Trial","Compute Time").unwrap();
  for i in 0..changes {
    nl += comp_nl[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,nl).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    ns += comp_ns[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,ns).unwrap();    
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
    is += comp_is[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}",i,is).unwrap();    
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
  writeln!(plotscript,"'{}' i 0 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat","Non-inc List").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat"," Non-inc Stack").unwrap();
  writeln!(plotscript,"'{}' i 2 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat","Inc List").unwrap();
  writeln!(plotscript,"'{}' i 3 u 1:2 t '{}' with lines,\\",filename.to_owned()+".dat","Inc Stack").unwrap();

  //generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
