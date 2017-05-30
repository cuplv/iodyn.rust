extern crate rand;
#[macro_use] extern crate clap;
extern crate adapton;
extern crate adapton_lab;
extern crate iodyn;
extern crate eval;

use std::io::{BufWriter,Write};
use std::fs::File;
use rand::{StdRng,SeedableRng};
use adapton::reflect;
//use adapton::engine::*;
//use adapton::macros::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
#[allow(unused)] use iodyn::{IRaz,IRazTree};
#[allow(unused)] use iodyn::archive_stack::{AtTail,AStack as IAStack};
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::eval_iastack::EvalIAStack;
#[allow(unused)] use eval::accum_lists::*;
#[allow(unused)] use eval::types::*;
use eval::actions::*;
use eval::test_seq::{TestResult,EditComputeSequence,aggregate};
use eval::util::*;

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

  //command-line
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
      -t, --trials=[trials]       'number of runs to aggregate stats from'
      -o, --outfile=[outfile]     'name for output files (of different extensions)'
      --trace                     'produce an output trace of the incremental run' ")
    .get_matches();
  let dataseed = value_t!(args, "data_seed", usize).unwrap_or(0);
  let editseed = value_t!(args, "edit_seed", usize).unwrap_or(0);
	let start_size = value_t!(args, "start", usize).unwrap_or(100_000);
	let datagauge = value_t!(args, "datagauge", usize).unwrap_or(1_000);
	let namegauge = value_t!(args, "namegauge", usize).unwrap_or(1);
	let edits = value_t!(args, "edits", usize).unwrap_or(1);
  let changes = value_t!(args, "changes", usize).unwrap_or(10);
  let trials = value_t!(args, "trials", usize).unwrap_or(1);
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
    comp: TreeFoldG::new(
      |ns:&Vec<usize>|{
        let mut sorted = ns.clone();
        sorted.sort();
        sorted
      },
      |t1,_,_,t2|{
        let mut o = Vec::with_capacity(t1.len()+t2.len());
        let mut i1 = t1.into_iter().peekable();
        let mut i2 = t2.into_iter().peekable();
        loop {
          let left = match (i1.peek(),i2.peek()) {
            (None, None) => break,
            (None, Some(_)) => false,
            (Some(_), None) => true,
            (Some(a), Some(b)) => {
              if a > b { false } else { true }
            },
          };
          if left { o.push(i1.next().unwrap()) }
          else { o.push(i2.next().unwrap()) }
        }
        o
      },
    ),
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
    comp: Native::new(|ns:&Vec<usize>|{
      let mut sorted = ns.clone();
      sorted.sort();
      sorted
    }),
    changes: changes,
  };


  // run experiments

  init_dcg(); assert!(engine_is_dcg());

  let mut rng = StdRng::from_seed(&[editseed]);

  let mut results_non_inc: Vec<TestResult<
    EvalVec<usize,StdRng>,
    Vec<_>,
  >> = Vec::new();
  let mut results_inc: Vec<TestResult<
    EvalIRaz<usize,StdRng>,
    //IRazTree<_>,
    Option<Vec<_>>,
  >> = Vec::new();

  for i in 0..trials+1 {
    //reseed rng
    let new_seed = &[dataseed+i];
    test_vec.init.coord.reseed(new_seed);
    test_raz.init.coord.reseed(new_seed);

    results_non_inc.push(test_vec.test(&mut rng));
    // for visual debugging
    if do_trace && i == 1 {reflect::dcg_reflect_begin()}

    results_inc.push(test_raz.test(&mut rng));

    if do_trace && i == 1 {
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
    // correctness check

    let non_inc_comparison =
      results_non_inc[i].result_data
      .clone().into_iter()
      .collect::<Vec<_>>()
    ;
    let inc_comparison =
      results_inc[i].result_data
      .clone().unwrap().into_iter()
      .collect::<Vec<_>>()
    ;
    if non_inc_comparison != inc_comparison {
      println!("Final results({}) differ:",i);
      println!("the incremental results({}): {:?}", inc_comparison.len(),inc_comparison);
      println!("non incremental results({}): {:?}", non_inc_comparison.len(),non_inc_comparison);
      println!("This is an error");
      ::std::process::exit(1);
    }
  }
  println!("Final results from all runs of both versions match.");

  let summary_inc = aggregate(&results_inc[1..]); // slice to remove warmup run
  let summary_non_inc = aggregate(&results_non_inc[1..]);

  println!("At input size: {}, Average of {} trials after warmup",start_size,trials);
  print_crossover_stats(&summary_non_inc.computes,&summary_inc.computes);

  // generate data file

  let filename = if let Some(f) = outfile {f} else {"out"};
  println!("Generating {}.pdf ...", filename);

  let mut dat = File::create(filename.to_owned()+".dat").unwrap();

  summary_non_inc.write_to(&mut dat);
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  summary_inc.write_to(&mut dat);

  // generate plot script

  let mut plotscript = File::create(filename.to_owned()+".plotscript").unwrap();

  writeln!(plotscript,"set terminal pdf").unwrap();
  writeln!(plotscript,"set output '{}.pdf'", filename).unwrap();
  write!(plotscript,"set title \"{}", "Cumulative time to sort after inserting element(s)\\n").unwrap();
  writeln!(plotscript,"(s)ize: {}, (g)auge: {}, (n)ame-gauge: {}, (e)dit-batch: {}, (t)rials: {}\"", start_size,datagauge,namegauge,edits,trials).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "(c)hanges").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"set grid ytics mytics  # draw lines for each ytics and mytics").unwrap();
  writeln!(plotscript,"set grid xtics mxtics  # draw lines for each xtics and mxtics").unwrap();
  writeln!(plotscript,"set mytics 5           # set the spacing for the mytics").unwrap();
  writeln!(plotscript,"set mxtics 5           # set the spacing for the mxtics").unwrap();
  writeln!(plotscript,"set grid               # enable the grid").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  writeln!(plotscript,"'{}.dat' i 0 u 1:6:7 ls 1 t '{}' with errorbars,\\",filename,"Non-incremental Compute Time").unwrap();
  writeln!(plotscript,"'{}.dat' i 0 u 1:6 ls 1 notitle with linespoints,\\",filename).unwrap();
  writeln!(plotscript,"'{}.dat' i 1 u 1:6:7 ls 2 t '{}' with errorbars,\\",filename,"Incremental Compute Time").unwrap();
  writeln!(plotscript,"'{}.dat' i 1 u 1:6 ls 2 notitle with linespoints,\\",filename).unwrap();

  // generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
