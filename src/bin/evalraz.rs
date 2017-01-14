extern crate rand;

#[macro_use]
extern crate clap;
extern crate time;
extern crate pmfp_collections;

use pmfp_collections::persist_raz as pr;
use pmfp_collections::gauged_raz as gr;

use rand::{StdRng, Rng, SeedableRng};

use time::{Duration};

//use pmfp_collections::zip::{Zip};
use pmfp_collections::seqzip::{Seq, SeqZip};

const DEFAULT_SEED: usize = 0;
const DEFAULT_TAG: &'static str = "None";
const DEFAULT_TAGHEAD: &'static str = "Tag";
const DEFAULT_START: usize = 0;
const DEFAULT_INSERT: usize = 10_000;
const DEFAULT_BATCH: usize = 1;
const DEFAULT_GROUPS: usize = 10;
const DEFAULT_REPS: usize = 1;

fn main() {
  //command-line
  let args = clap::App::new("evalraz")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Evaluator (and eventually tester) for RAZ data structure")
    .args_from_usage("\
      --nohead               'supress csv header'
      --seed=[seed]          'random seeds'
      --tag=[tag]            'user tag'
      --taghead=[taghead]    'header title for tag'
      --save_mem             'don't dealocate major data while timing'
      -s, --start=[start]    'starting sequence length'
      -i, --insert=[insert]  'number of timed insertions'
      -b, --batch=[batch]    'number of consequtive insertions before reposition'
      -g, --groups=[groups]  'measured insertion groups per sequence'
      -r, --reps=[reps]      'number of sequences tested'
      [multi] -m             'more insertions for each repetition'
      [raz] -z               'test raz'
      [graz] -Z              'test gauged raz zip interface'
      [mraz] -R              'test gauged raz mut interface' ")
    .get_matches();
  let nohead = args.is_present("nohead");
  let seed = value_t!(args, "seed", usize).unwrap_or(DEFAULT_SEED);
  let tag = args.value_of("tag").unwrap_or(DEFAULT_TAG);
  let taghead = args.value_of("taghead").unwrap_or(DEFAULT_TAGHEAD);
  let save_mem = args.is_present("save_mem");
	let start = value_t!(args, "start", usize).unwrap_or(DEFAULT_START);
	let insert = value_t!(args, "insert", usize).unwrap_or(DEFAULT_INSERT);
	let batch = value_t!(args, "batch", usize).unwrap_or(DEFAULT_BATCH);
	let groups = value_t!(args, "groups", usize).unwrap_or(DEFAULT_GROUPS);
	let reps = value_t!(args, "reps", usize).unwrap_or(DEFAULT_REPS);
  let multi = args.is_present("multi");
  let mut eval_raz = args.is_present("raz");
  let eval_graz = args.is_present("graz");
  let eval_mraz = args.is_present("mraz");

  // extend this with other evaluations in the future so we always do at least one
  if !eval_raz && !eval_graz && !eval_mraz {
  	eval_raz = true;
  }

	let print_header = ||{
	   println!("UnixTime,Seed,SeqType,SeqNum,PriorElements,Insertions,Batch,Time,{}", taghead);
	};

	let print_result = |version: &str, number: usize, prior_elms: usize, insertions: usize, time: Duration| {
		println!("{},{},{},{},{},{},{},{},{}",
			time::get_time().sec, seed, version, number, prior_elms, insertions, batch, time, tag
		);
	};

  // make empty sequences
  let mut raz_start = pr::Raz::new();
  let mut graz_start = gr::Raz::new();
  let mut mraz_start = gr::Raz::new();

  // print header
  if !nohead { print_header() }

	// initialize with starting elements
	if start > 0 {
		if eval_raz {
			let start_time = time::get_time();
			raz_start = insert_n(raz_start, start, 0, StdRng::from_seed(&[seed]));
			let elapsed = time::get_time() - start_time;
			print_result("RAZ", 0, 0, start, elapsed);
		}
		if eval_graz {
			let start_time = time::get_time();
			graz_start = insert_n(graz_start, start, 0, StdRng::from_seed(&[seed]));
			let elapsed = time::get_time() - start_time;
			print_result("GRAZ", 0, 0, start, elapsed);
		}
		if eval_mraz {
			let start_time = time::get_time();
			mraz_start = insert_n_mut(mraz_start, start, 0, StdRng::from_seed(&[seed]));
			let elapsed = time::get_time() - start_time;
			print_result("MRAZ", 0, 0, start, elapsed);
		}
	}

	// run tests
  for i in 0..reps {
  	let ins = if multi {insert * i} else {insert};

  	// raz
  	if eval_raz {
  		if save_mem {
  			let mut seqs = Vec::new();
  			let mut zips = Vec::new();
		  	let mut raz_size = start;
		  	let mut build_raz = raz_start.clone();
		  	for _ in 0..groups {
					let start_time = time::get_time();
		  		let (new_raz,new_seqs,new_zips) = insert_n_save(build_raz, ins, raz_size, StdRng::from_seed(&[seed]),seqs,zips);
					let elapsed = time::get_time() - start_time;
	  			print_result("RAZ",i,raz_size,ins,elapsed);
  				build_raz = new_raz;
  				seqs = new_seqs;
  				zips = new_zips;
		  		raz_size += ins;
  			}
  		} else {
		  	let mut raz_size = start;
		  	let mut build_raz = raz_start.clone();
		  	for _ in 0..groups {
					let start_time = time::get_time();
		  		build_raz = insert_n_batch(build_raz, ins, batch, raz_size, StdRng::from_seed(&[seed]));
					let elapsed = time::get_time() - start_time;
	  			print_result("RAZ",i,raz_size,ins,elapsed);
		  		raz_size += ins;
		  	}
		  }
	  }
	  // gauged raz zip
	  if eval_graz {
	  	let mut graz_size = start;
	  	let mut build_graz = graz_start.clone();
	  	for _ in 0..groups {
				let start_time = time::get_time();
	  		build_graz = insert_n(build_graz, ins, graz_size, StdRng::from_seed(&[seed]));
				let elapsed = time::get_time() - start_time;
  			print_result("GRAZ",i,graz_size,ins,elapsed);
	  		graz_size += ins;
	  	}
	  }
	  // gauged raz mut
	  if eval_mraz {
	  	let mut mraz_size = start;
	  	let mut build_mraz = mraz_start.clone();
	  	for _ in 0..groups {
				let start_time = time::get_time();
	  		build_mraz = insert_n_batch_mut(build_mraz, ins, batch, mraz_size, StdRng::from_seed(&[seed]));
				let elapsed = time::get_time() - start_time;
  			print_result("MRAZ",i,mraz_size,ins,elapsed);
	  		mraz_size += ins;
	  	}
	  }

  }
}

// insert into seq `zip` (of current size `length`)
// `n` elements into seperate random positions
// each elemtent is the length at the time of insertion
fn insert_n<Z: SeqZip<usize,S>, S: Seq<usize,Z>>(zip: Z, n: usize, size: usize, mut rnd_pos: StdRng) -> Z {
	let mut zip: Z = zip;
	let mut seq: S;
	for i in 0..n {
    let pos = rnd_pos.gen::<usize>() % (size + 1 + i);
    seq = zip.unzip();
    zip = seq.zip_to(pos).unwrap();
    zip = zip.push_r(size + i);
	}
	zip
}
fn insert_n_batch<Z: SeqZip<usize,S>, S: Seq<usize,Z>>(zip: Z, n: usize, b:usize, size: usize, mut rnd_pos: StdRng) -> Z {
	let mut zip: Z = zip;
	let mut seq: S;
	for i in 0..(n/b) {
    let pos = rnd_pos.gen::<usize>() % (size + 1 + i * b);
    seq = zip.unzip();
    zip = seq.zip_to(pos).unwrap();
    for j in 0..b {
	    zip = zip.push_r(size + i * b + j);
    }
	}
	zip
}
fn insert_n_mut(mut raz: gr::Raz<usize>, n: usize, size: usize, mut rnd_pos: StdRng) -> gr::Raz<usize> {
	for i in 0..n {
    let pos = rnd_pos.gen::<usize>() % (size + 1 + i);
    raz = raz.unfocus().focus(pos).unwrap();
	  raz.push_right(size + i);	
	}
  raz
}
fn insert_n_batch_mut(mut raz: gr::Raz<usize>, n: usize, b: usize, size: usize, mut rnd_pos: StdRng) -> gr::Raz<usize> {
	for i in 0..(n/b) {
    let pos = rnd_pos.gen::<usize>() % (size + 1 + i * b);
    raz = raz.unfocus().focus(pos).unwrap();
    for j in 0..(b) {
	    raz.push_right(size + i * b + j);	
    }
	}
  raz
}

// same as above but saves all data in vecs to avoid dealocations during timing
fn insert_n_save<Z: SeqZip<usize,S>, S: Seq<usize,Z>>(
	zip: Z, n: usize, size: usize, mut rnd_pos: StdRng, mut seqs: Vec<S>, mut zips: Vec<Z>
) -> (Z,Vec<S>,Vec<Z>) {
	let mut zip: Z = zip;
	for i in 0..n {
    let pos = rnd_pos.gen::<usize>() % (size + 1 + i);
    let seq = zip.unzip();
    zips.push(zip);
    zip = seq.zip_to(pos).unwrap();
    seqs.push(seq);
    let new_zip = zip.push_r(size + i);
    zips.push(zip);
    zip = new_zip;
	}
	(zip,seqs,zips)
}
