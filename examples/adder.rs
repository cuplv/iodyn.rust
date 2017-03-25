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
use rand::{Rand,Rng,StdRng,SeedableRng};
use eval::actions::*;
use eval::interface::{IntrfSeq,IntrfNew,IntrfArchive};
#[allow(unused)] use pmfp_collections::IRaz;
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
use eval::test_seq::{TestMResult,EditComputeSequence};
use adapton::engine::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;

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

  let child =
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}
fn main2() {

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

	fn tokenize_step<A: IntrfSeq<Token>>((ts,part):(A,Option<u32>),l:&Lang) -> (A,Option<u32>) {
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
	fn tokenize_meta<A: IntrfArchive<(u32,Option<Name>)>>((ts,part):(A,Option<u32>),(l,n):(u32,Option<Name>)) -> (A,Option<u32>) {
		(ts.archive((l,n.map(|n|{
			name_pair(n,name_of_string(String::from("tok_accum")))
		}))),part)
	}
	fn tokenize_final<A: IntrfSeq<Token>>((ts,part):(A,Option<u32>)) -> A {
		if let Some(num) = part {
			ts.seq_push(Token::Num(num))
		} else { ts }
	}
	fn parse_step<A: IntrfSeq<u32>>(n: A, t: &Token) -> A {
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
	fn parse_meta<A: IntrfArchive<(u32,Option<Name>)>>(num: A, (l,n): (u32,Option<Name>)) -> A {
		num.archive((l,n.map(|n|{
			name_pair(n,name_of_string(String::from("par_accum")))
		})))
	}
  let mut test = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    // The type here determins the type of the accumulator
    // TODO: Move this parameter elsewhere
    comp: Compute2::<_,_,_,EvalIRaz<Token,StdRng>,_>::new(
    	MFolder::new(
    		name_of_string(String::from("tokenize")),
				(IntrfNew::new(),None),
				tokenize_step,
				tokenize_meta,
				|a|{
					let ts = tokenize_final(a);
					IncrementalFrom{
						data: ts,
			      unitgauge: unitgauge,
			      namegauge: namegauge,
			      coord: coord.clone(),
					}.create(&mut StdRng::new().unwrap()).1
				},
			),
    	MFolder::new(
    		name_of_string(String::from("parse")),
    		IntrfNew::new(),
    		parse_step,
    		parse_meta,
    		|a|{a},
    	)
		),
    changes: changes,
  };

  init_dcg(); assert!(engine_is_dcg());
  // for visual debugging
  reflect::dcg_reflect_begin();

  // run experiments
  let mut rng = StdRng::from_seed(&[editseed]);
  let result: TestMResult<
  	EvalIRaz<Lang,StdRng>, // in type
  	IRaz<u32>,  // out type
  > = test.test(&mut rng);
  // let result: TestMResult<
  // 	EvalVec<Lang,StdRng>,
  // 	Vec<u32>,
  // > = test.test(&mut rng);

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

  println!("inc times(ns) (tokenize,parse): {:?}", result.computes.iter().map(|c|{
  	(c[0].num_nanoseconds().unwrap(),c[1].num_nanoseconds().unwrap())
  }).collect::<Vec<_>>());

  // TODO: chart results

}
