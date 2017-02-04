//! This is a system for creating charts of the
//! performance of various forms of the raz
//! data structures defined in this crate

extern crate rand;
extern crate time;
#[macro_use] extern crate clap;
extern crate stats;
extern crate adapton;
extern crate pmfp_collections;

mod eval;

use time::{Duration};
//use adapton::engine::*;
//use pmfp_collections::trees::{NegBin};

const DEFAULT_DATASEED: usize = 0;
const DEFAULT_EDITSEED: usize = 0;
const DEFAULT_TAG: &'static str = "None";
const DEFAULT_TAGHEAD: &'static str = "Tag";
const DEFAULT_START: usize = 0;
const DEFAULT_UNITSIZE: usize = 10;
const DEFAULT_NAMESIZE: usize = 0;
const DEFAULT_EDITS: usize = 1;
const DEFAULT_BATCHES: usize = 1;
const DEFAULT_CHANGES: usize = 10;
const DEFAULT_TRIALS: usize = 10;
const DEFAULT_VARY: &'static str = "none";

enum WhichVary {
	Nil,S,U,N,E,B,C
}
use WhichVary::*;

pub struct Params {
	dataseed: [usize;1],
	start: usize,
	unitsize: usize,
	namesize: usize,
	edits: usize,
	batches: usize,
	changes: usize,
	trials: usize,
}

fn main() {
  //command-line
  let args = clap::App::new("chartraz")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Produces comparison charts for RAZ data structure")
    .args_from_usage("\
      --nohead                  'supress header'
      --dataseed=[dataseed]			'seed for random data'
      --editseed=[edit_seed]    'seed for random edits (and misc.)'
      --tag=[tag]               'user tag'
      --taghead=[taghead]       'header title for tag'
      -s, --start=[start]       'starting sequence length'
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -b, --batches=[batches]   'batches per incremental change'
      -c, --changes=[changes]   'number of incremental changes'
      -t, --trials=[trials]     'trials to average over'
      --vary=[vary]             'parameter to vary (one of sunebc, adjust x2)' ")
    .get_matches();
  let nohead = args.is_present("nohead");
  let dataseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_DATASEED);
  let editseed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_EDITSEED);
  let tag = args.value_of("tag").unwrap_or(DEFAULT_TAG);
  let taghead = args.value_of("taghead").unwrap_or(DEFAULT_TAGHEAD);
	let start = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let unitsize = value_t!(args, "unitsize", usize).unwrap_or(DEFAULT_UNITSIZE);
	let namesize = value_t!(args, "namesize", usize).unwrap_or(DEFAULT_NAMESIZE);
	let edits = value_t!(args, "edits", usize).unwrap_or(DEFAULT_EDITS);
	let batches = value_t!(args, "batches", usize).unwrap_or(DEFAULT_BATCHES);
	let changes = value_t!(args, "changes", usize).unwrap_or(DEFAULT_CHANGES);
	let trials = value_t!(args, "trials", usize).unwrap_or(DEFAULT_TRIALS);
	let vary = match args.value_of("vary").unwrap_or(DEFAULT_VARY) {
		"none"=>Nil,"s"=>S,"u"=>U,"n"=>N,"e"=>E,"b"=>B,"c"=>C,
		_ => panic!("vary takes on of: s,u,n,e,b,c")
	};

	let print_header = ||{
	   println!("Timestamp,Seed,SeqType,SeqNum,PriorElements,Insertions,Batches,Time,{}", taghead);
	};

	let print_result = |version: &str, number: usize, prior_elms: usize, insertions: usize, time: Duration| {
		println!("{},{},{},{},{},{},{},{},{}",
			time::get_time().sec, dataseed, version, number, prior_elms, insertions, batches, time, tag
		);
	};

  // print header
  if !nohead { print_header() }

	// initialize with starting elements
	if start > 0 {
	}

}


