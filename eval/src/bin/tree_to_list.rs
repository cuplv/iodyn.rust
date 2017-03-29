extern crate pmfp_collections;
extern crate adapton;
extern crate eval;
extern crate time;
extern crate rand;
#[macro_use] extern crate clap;

extern crate adapton_lab;

// use std::fs::OpenOptions;
// use std::io::Write;
// use time::Duration;
use rand::{StdRng,SeedableRng};
use eval::actions::*;
use eval::interface::{IFaceSeq,IFaceNew,IFaceArchive};
#[allow(unused)] use pmfp_collections::{IRaz,IRazTree};
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::eval_iastack::EvalIAStack;
use eval::test_seq::{TestMResult,EditComputeSequence};
use eval::types::*;
use pmfp_collections::inc_gauged_raz::{AtTail};
use pmfp_collections::inc_archive_stack::AStack as IAStack;
use adapton::engine::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;

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

  init_dcg(); assert!(engine_is_dcg());
  // for visual debugging
  reflect::dcg_reflect_begin();

  // run experiments
  let mut rng = StdRng::from_seed(&[editseed]);
  let result: TestMResult<
  	EvalIRaz<GenSmall,StdRng>,  // in type
  	IAStack<GenSmall,u32>, // out type
  > = test.test(&mut rng);

  // for visual debugging
  let traces = reflect::dcg_reflect_end();
  let f = File::create("trace.html").unwrap();
  let mut writer = BufWriter::new(f);
  writeln!(writer, "{}", style_string()).unwrap();
  writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
  writeln!(writer, "<div class=\"traces\">").unwrap();
  for tr in traces {
  	div_of_trace(&tr).write_html(&mut writer);
  }

  println!("inc times(ns): {:?}", result.computes.iter().map(|c|{
  	c[0].num_nanoseconds().unwrap()
  }).collect::<Vec<_>>());

  // TODO: chart results

}
