//! Test of a non-trivial fold over a tree and conversion from list to tree
//! 
//! This test takes a randomized input string, tokenizes it, and evaluates
//! the tokens to produce a list of numbers. The calculation is a prefix 
//! addition of numbers.
//! 
//! The test is preformed with an incremental tree and sequence combination,
//! or with a vector(array) to test non-incremental performance.

extern crate rand;
#[macro_use] extern crate clap;
extern crate adapton;
extern crate adapton_lab;
extern crate pmfp_collections;
extern crate eval;

use std::io::BufWriter;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use rand::{Rand,Rng,StdRng,SeedableRng};
use adapton::engine::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
#[allow(unused)] use pmfp_collections::IRaz;
#[allow(unused)] use pmfp_collections::inc_archive_stack::{AtTail,AStack as IAStack};
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::eval_iastack::EvalIAStack;
#[allow(unused)] use eval::accum_lists::*;
use eval::actions::*;
use eval::interface::{IFaceSeq,IFaceNew,IFaceArchive};
use eval::test_seq::{TestMResult,EditComputeSequence};

/// Input Lang
///
/// Unchecked, but should only contain [0123456789,+]
/// This is mainly used to randomly generate a sequence
#[derive(Debug,Clone,Copy,Hash,Eq,PartialEq)]
struct Lang(pub char);
impl Rand for Lang{
  fn rand<R: Rng>(rng: &mut R) -> Self {
  	Lang(match rng.gen::<usize>() % 7 {
  		0 => '+',
  		1...3 => ',',
  		_ => *rng.choose(&['0','1','2','3','4','5','6','7','8','9']).unwrap(),
  	})
  }
}

/// Intermediate Lang
#[derive(Debug,Clone,Copy,Hash,Eq,PartialEq)]
enum Token {
	Num(u32),
	OpPlus,
}

/// Reference parser and tokenizer for later work
#[allow(unused)]
mod reference {
	use super::{Token};

	fn tokenize(p: String) -> Vec<Token> {
		p.chars().fold((Vec::new(),None),|(mut ts,part),c|{
			if let Some(num) = c.to_digit(10) {
  			// this multiplication might overflow!
				let bigger = part.map_or(num,|p|p*10+num);
				(ts,Some(bigger))
			} else {
				if let Some(num) = part {
					ts.push(Token::Num(num));
				}
				match c {
					'+' => ts.push(Token::OpPlus),
					',' => {},
					c => panic!("invalid char '{}'",c),
				}
				(ts,None)
			}
		}).0
	}

	fn parse(toks: Vec<Token>) -> Vec<u32> {
		toks.iter().fold(Vec::new(),|mut n,t|{
			match *t {
				Token::Num(a) => n.push(a),
				Token::OpPlus => {
					if let Some(b) = n.pop() {
						if let Some(a) = n.pop() {
							n.push(a+b);
						} else {
							n.push(b);
						}
					}
				}
			}
			n
		})
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		#[test]
		fn test_ref_impl() {
			let prog = "23+4+4,23,54,123,5++".to_string();
			let toks = tokenize(prog);
			let vals = parse(toks);
			println!("{:?}", vals);
			assert_eq!(vec![27, 4, 23, 182], vals);
		}
	}
}

//////////////////////////////
// Test of incremental version
//////////////////////////////

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
  let args = clap::App::new("adder")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .about("Parsing Example")
    .args_from_usage("\
      --dataseed=[dataseed]			'seed for random data'
      --editseed=[edit_seed]    'seed for random edits (and misc.)'
      -s, --start=[start]       'starting sequence length'
      -u, --unitsize=[unitsize] 'initial elements per structure unit'
      -n, --namesize=[namesize] 'initial tree nodes between each art'
      -e, --edits=[edits]       'edits per batch'
      -c, --changes=[changes]   'number of incremental changes'
      -o, --outfile=[outfile]   'name for output files (of different extensions)'
      --trace                   'produce an output trace of the incremental run' ")
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

	// fold function that builds a sequece of tokens
	fn tokenize_step<A: IFaceSeq<Token>>((ts,part):(A,Option<u32>),l:&Lang) -> (A,Option<u32>) {
		let Lang(ref c) = *l;
		if let Some(num) = c.to_digit(10) {
			let bigger = part.map_or(num,|p|p*10+num);
			// we need to split up big nums, so use max 6 digits
			let (ts,bigger) = if bigger > 99999 {
				(ts.seq_push(Token::Num(bigger)),num)
			} else {(ts,bigger)};
			(ts,Some(bigger))
		} else {
			let ts = if let Some(num) = part {
				ts.seq_push(Token::Num(num))
			} else { ts };
			let ts = match *c {
				'+' => {ts.seq_push(Token::OpPlus)},
				',' => {ts},
				c => panic!("invalid char '{}'",c),
			};
			(ts,None)
		}
	}
	// auxiliary fold function that inserts incremental names
	fn tokenize_meta<A: IFaceArchive<(u32,Option<Name>)>>((ts,part):(A,Option<u32>),(l,n):(u32,Option<Name>)) -> (A,Option<u32>) {
		(ts.archive((l,n)),part)
	}
	// auxiliary fold function that completes the token sequence
	fn tokenize_final<A: IFaceSeq<Token>>((ts,part):(A,Option<u32>)) -> A {
		if let Some(num) = part {
			ts.seq_push(Token::Num(num))
		} else { ts }
	}
	// fold function that evaluates tokens to build a sequence of numbers
	fn parse_step<A: IFaceSeq<u32>>(n: A, t: &Token) -> A {
		match *t {
			Token::Num(a) => n.seq_push(a),
			Token::OpPlus => {
				let (b,n) = n.seq_pop();
				if let Some(b) = b {
					let (a,n) = n.seq_pop();
					if let Some(a) = a {
						n.seq_push(a+b)
					} else {
						n.seq_push(b)
					}
				} else { n }
			}
		}
	}
	// auxiliary fold function that inserts incremental names
	fn parse_meta<A: IFaceArchive<(u32,Option<Name>)>>(num: A, (l,n): (u32,Option<Name>)) -> A {
		num.archive((l,n))
	}
	let name_to_tree = name_of_string(String::from("to_tree"));
	
	// Test parameters - two nearly identical sets, differing
	// by the type of accumulator used between the tokenize
	// and parse stages
  let mut test_inc = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    // The type here determines the type of data between the
    // two computations (output from 1 = input to 2).
    // The specific accumulator is determined below in the
    // conversion to a test harness.
    // TODO: Move this parameter elsewhere
    comp: Compute2::<_,_,_,EvalIRaz<Token,StdRng>,_>::new(
    	MFolder::new(
    		name_of_string(String::from("tokenize")),
				(IFaceNew::new(),None),
				tokenize_step,
				tokenize_meta,
				|a|{
					let ts = tokenize_final(a);
					ns(name_to_tree.clone(),||{IncrementalFrom{
						data: AtTail(ts), // This determines the accumulator type
			      unitgauge: unitgauge,
			      namegauge: namegauge,
			      coord: coord.clone(),
					}.create(&mut StdRng::new().unwrap()).1})
				},
			),
    	MFolder::new(
    		name_of_string(String::from("parse")),
    		IFaceNew::new(),
    		parse_step,
    		parse_meta,
    		|a|{a},
    	)
		),
    changes: changes,
  };
  let mut test_non_inc = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    // The type here determines the type of data between the
    // two computations (output from 1 = input to 2).
    // The specific accumulator is determined below in the
    // conversion to a test harness.
    // TODO: Move this parameter elsewhere
    comp: Compute2::<_,_,_,EvalVec<Token,StdRng>,_>::new(
    	MFolder::new(
    		name_of_string(String::from("tokenize")),
				(IFaceNew::new(),None),
				tokenize_step,
				tokenize_meta,
				|a|{
					let ts = tokenize_final(a);
					ns(name_to_tree.clone(),||{IncrementalFrom{
						data: ts, // This determines the accumulator type
			      unitgauge: unitgauge,
			      namegauge: namegauge,
			      coord: coord.clone(),
					}.create(&mut StdRng::new().unwrap()).1})
				},
			),
    	MFolder::new(
    		name_of_string(String::from("parse")),
    		IFaceNew::new(),
    		parse_step,
    		parse_meta,
    		|a|{a},
    	)
		),
    changes: changes,
  };

  init_dcg(); assert!(engine_is_dcg());


  // run experiments

  let mut rng = StdRng::from_seed(&[editseed]);

  let result_non_inc: TestMResult<
  	EvalVec<Lang,StdRng>,
  	Vec<u32>,
  > = test_non_inc.test(&mut rng);

  // for visual debugging
  if do_trace {reflect::dcg_reflect_begin()}

  let result_inc: TestMResult<
  	EvalIRaz<Lang,StdRng>, // in type
  	IAStack<u32,u32>,  // out type
  > = test_inc.test(&mut rng);

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

  // post-process results


  let edit_non_inc = result_non_inc.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let edit_inc = result_inc.edits.iter().map(|d|d.num_nanoseconds().unwrap()).collect::<Vec<_>>();
  let comp_non_inc = result_non_inc.computes.iter().map(|d|(d[0].num_nanoseconds().unwrap(),d[1].num_nanoseconds().unwrap())).collect::<Vec<_>>();
  let comp_inc = result_inc.computes.iter().map(|d|(d[0].num_nanoseconds().unwrap(),d[1].num_nanoseconds().unwrap())).collect::<Vec<_>>();
  
  println!("computes_non_inc(tokenize,parse): {:?}", comp_non_inc);
  println!("computes_inc(tokenize,parse): {:?}", comp_inc);


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

  let (mut en,mut tn,mut pn,mut ei,mut ti,mut pi) = (0f64,0f64,0f64,0f64,0f64,0f64);
  writeln!(dat,"'{}'\t'{}'\t'{}'\t'{}'","Changes","Edit Time","Tokenize Time","Parse Time").unwrap();
  for i in 0..changes {
    en += edit_non_inc[i] as f64 / 1_000_000.0;
    tn += comp_non_inc[i].0 as f64 / 1_000_000.0;
    pn += comp_non_inc[i].1 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,en,tn,pn).unwrap();    
  }
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  for i in 0..changes {
    ei += edit_inc[i] as f64 / 1_000_000.0;
    ti += comp_inc[i].0 as f64 / 1_000_000.0;
    pi += comp_inc[i].1 as f64 / 1_000_000.0;
    writeln!(dat,"{}\t{}\t{}\t{}",i,ei,ti,pi).unwrap();    
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
  write!(plotscript,"set title \"{}", "Accumulating time to insert element(s) and parse\\n").unwrap();
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
  writeln!(plotscript,"'{}' i 0 u 1:($3+$4) t '{}' with linespoints,\\",filename.to_owned()+".dat","Non-incremental Parse Time").unwrap();
  writeln!(plotscript,"'{}' i 1 u 1:($3+$4) t '{}' with linespoints,\\",filename.to_owned()+".dat","Incremental Parse Time").unwrap();

  //generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
