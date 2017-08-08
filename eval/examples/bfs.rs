extern crate rand;
#[macro_use] extern crate clap;
#[macro_use] extern crate adapton;
extern crate adapton_lab;
extern crate iodyn;
extern crate eval;

use std::rc::Rc;
use std::io::BufWriter;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashMap;
use rand::{StdRng,SeedableRng};
use adapton::reflect;
use adapton::engine::*;
use adapton::macros::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
#[allow(unused)] use iodyn::{IRaz,IRazTree};
#[allow(unused)] use iodyn::archive_stack::{AtTail,AStack as IAStack};
use iodyn::finite_map::*;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::eval_iastack::EvalIAStack;
#[allow(unused)] use eval::accum_lists::*;
#[allow(unused)] use eval::types::*;
#[allow(unused)] use eval::eval_hashmap::EvalHashMap;
use eval::actions::*;
use eval::test_seq::{TestResult,EditComputeSequence};

fn main () {

// provide additional stack memory
  let child =
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}
fn main2() {
// end provide additional stack memory

let args = clap::App::new("quickhull")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .args_from_usage("\
      --dataseed=[dataseed]			  'seed for random data'
      --editseed=[edit_seed]      'seed for random edits (and misc.)'
      -s, --start=[start]         'starting sequence length'
      -g, --datagauge=[datagauge] 'initial elements per structure unit'
      -n, --namegauge=[namegauge] 'initial tree nodes between each art'
      -e, --edits=[edits]         'edits per batch'
      -c, --changes=[changes]     'number of incremental changes'
      -o, --outfile=[outfile]     'name for output files (of different extensions)'
      --trace                     'produce an output trace of the incremental run' ")
    .get_matches();
  let dataseed = value_t!(args, "data_seed", usize).unwrap_or(0);
  let editseed = value_t!(args, "edit_seed", usize).unwrap_or(0);
	let start_size = value_t!(args, "start", usize).unwrap_or(1_000_000);
	let datagauge = value_t!(args, "datagauge", usize).unwrap_or(1_000);
	let namegauge = value_t!(args, "namegauge", usize).unwrap_or(1);
	let edits = value_t!(args, "edits", usize).unwrap_or(1);
	let changes = value_t!(args, "changes", usize).unwrap_or(30);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace");
	let coord = StdRng::from_seed(&[dataseed]);

let mut test_raz = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: Native::new(|g:&IRazTree<Option<(Option<usize>,Vec<usize>)>>| {
	    let gsized = SizedMap { size: start_size*2, gran: datagauge, map: *g };
	    let ret: SizedMap<(Option<usize>, Vec<usize>)> = Graph::bfs(gsized, 1);
	    ret.map
    }),
    changes: changes,
  };

let mut test_vec = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: Native::new(|g:&HashMap<usize, (Option<usize>, Vec<usize>)>| {
	    let gsized = SizedHashMap { size: 1000000, gran: 10000, map: *g };
	    let ret: SizedHashMap<usize, (Option<usize>, Vec<usize>)> = Graph::bfs(gsized, 1);
    	ret.map
    }),
    changes: changes,
  };

	init_dcg(); assert!(engine_is_dcg());

  // run experiments

  let mut rng = StdRng::from_seed(&[editseed]);

  let result_non_inc: TestResult<
    EvalHashMap<(Option<usize>, Vec<usize>),StdRng>,
    HashMap<_, _>,
  > = test_vec.test(&mut rng);

  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin()}

  let result_inc: TestResult<
    EvalIRaz<Option<(Option<usize>,Vec<usize>)>,StdRng>,
    IRazTree<Option<(Option<usize>,Vec<usize>)>>,
  > = test_raz.test(&mut rng);

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

/*correctness check malformed for this case
  // correctness check

  let non_inc_comparison =
    result_non_inc.result_data
    .clone().into_iter()
    .collect::<Vec<_>>()
  ;
  let inc_comparison =
    result_inc.result_data
    .clone().into_iter()
    .collect::<Vec<_>>()
  ;
  match non_inc_comparison == inc_comparison {
    true => println!("Final results from both versions match"),
    false => {
      println!("Final results differ:");
      println!("the incremental results({}): {:?}", inc_comparison.len(),inc_comparison);
      println!("non incremental results({}): {:?}", non_inc_comparison.len(),non_inc_comparison);
      println!("This is an error");
      ::std::process::exit(1);
    }
  }*/

  // post-process results

  let edit_non_inc = result_non_inc.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_inc = result_inc.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_non_inc = result_non_inc.computes.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_inc = result_inc.computes.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  
  let mut adapton_changes = Vec::new();
  let mut native_changes = Vec::new();
  for i in 0..(changes+1) {
    let nc = comp_non_inc[i];
    native_changes.push(nc as f64 / 1_000_000.0);
    let ac = comp_inc[i];
    adapton_changes.push(ac as f64 / 1_000_000.0);
  }
  let adapton_init = adapton_changes[0];
  let native_init = native_changes[0];
  adapton_changes.remove(0);
  native_changes.remove(0);

  let update_time = adapton_changes.iter().sum::<f64>() / adapton_changes.len() as f64;
  let crossover = native_changes.iter().zip(adapton_changes.iter()).enumerate()
    .fold((native_init,adapton_init,0),|(n,a,cross),(c,(nt,at))|{
      let new_n = n + nt;
      let new_a = a + at;
      let new_cross = if n < a && new_n >= new_a { c + 1 } else { cross };
      (new_n,new_a,new_cross)
    }).2;

  println!("At input size: {}",start_size);
  println!(" - Native initial run: {:.*} ms",2,native_init);
  println!(" - Adapton initial run: {:.*} ms",2,adapton_init);
  println!(" - Adapton overhead: {:.*} (Adapton initial time / Native initial time)",2,adapton_init/native_init);
  println!(" - Adapton update time: {:.*} ms avg over the first {} changes",2,update_time,changes);
  if crossover > 0 {
    println!(" - Adapton cross over: {} changes  (When Adapton's update time overcomes its overhead)",crossover);
  }  else {
    println!(" - Adapton cross over off chart  (When Adapton's update time overcomes its overhead)");
  }
  println!(" - Adapton speedup: {:.*} (Native initial time / Adapton update time)",2,native_init/update_time);


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

  let (mut en,mut rn,mut ei,mut ri) = (0f64,0f64,0f64,0f64);
  writeln!(dat,"'{}'\t'{}'\t'{}'","Changes","Edit Time","Compute Time").unwrap();
  for i in 0..changes {
    en += edit_non_inc[i] as f64 / 1_000_000.0;
    rn += comp_non_inc[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}",i,en,rn).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    ei += edit_inc[i] as f64 / 1_000_000.0;
    ri += comp_inc[i] as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}",i,ei,ri).unwrap();    
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
  write!(plotscript,"set title \"{}", "Cumulative time to calculate quickhull after inserting element(s)\\n").unwrap();
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
  writeln!(plotscript,"'{}' i 0 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Non-incremental Compute Time").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:3 t '{}' with linespoints,\\",filename.to_owned()+".dat","Incremental Compute Time").unwrap();

  //generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}