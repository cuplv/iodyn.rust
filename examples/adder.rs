extern crate pmfp_collections;
extern crate adapton;
extern crate eval;
extern crate time;
extern crate rand;

// use std::fs::OpenOptions;
// use std::io::Write;
// use time::Duration;
use rand::{Rand,Rng,StdRng,SeedableRng};
use eval::actions::*;
use eval::primitives::{EditSeq};
#[allow(unused)] use eval::eval_nraz::EvalNRaz;
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
use eval::test_seq::{TestMResult,EditComputeSequence};
use adapton::engine::manage::*;

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

// reference tokenizer
#[allow(unused)]
fn tokenize(p: String) -> Vec<Token> {
	p.chars().fold((Vec::new(),None),|(mut ts,part),c|{
		if let Some(num) = c.to_digit(10) {
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

// reference parser
#[allow(unused)]
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

//////////////////////////////
// Test of incremental version
//////////////////////////////

fn main() {
	let unitgauge = 1000;
	let namegauge = 1;
	let coord = StdRng::from_seed(&[0]);
	let mut accum = IncrementalEmpty{
		unitgauge: unitgauge,
		namegauge: namegauge,
		coord: coord.clone(),
	};
	let unused_rng = &mut StdRng::from_seed(&[0]);
	fn tokenize_step<A: EditSeq<Token>>(ts:A,part:Option<u32>,l:&Lang) -> (A,Option<u32>) {
		let Lang(ref c) = *l;
		if let Some(num) = c.to_digit(10) {
			let bigger = part.map_or(num,|p|p*10+num);
			(ts,Some(bigger))
		} else {
			let ts = if let Some(num) = part {
				ts.push(Token::Num(num)).1
			} else { ts };
			let ts = match *c {
				'+' => {ts.push(Token::OpPlus).1},
				',' => {ts},
				c => panic!("invalid char '{}'",c),
			};
			(ts,None)
		}
	}
	fn parse_step<A: EditSeq<u32>>(n: A, t: &Token) -> A {
		match *t {
			Token::Num(a) => n.push(a).1,
			Token::OpPlus => {
				let (_,b,n) = n.pop();
				if let Some(b) = b {
					let (_,a,n) = n.pop();
					if let Some(a) = a {
						n.push(a+b).1
					} else {
						n.push(b).1
					}
				} else { n }
			}
		}
	}
  let mut test = EditComputeSequence{
    init: IncrementalInit {
      size: 1_000,
      unitgauge: unitgauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(1),
    comp: Compute2::<_,_,_,EvalVec<Token,StdRng>,_>::new(
    	Compute2::new(Folder::new(
				(accum.create(unused_rng).1,None),
				|(ts,part),l:&Lang|{tokenize_step(ts,part,l)},
			), Proj0),
    	Folder::new(
				accum.create(unused_rng).1,
				|n,t|{parse_step(n,t)},
			)
		),
    changes: 30,
  };

  let _ = init_dcg(); assert!(engine_is_dcg());

  // run experiments
  let mut rng = StdRng::from_seed(&[0]);
  // let result: TestMResult<
  // 	EvalIRaz<Lang,StdRng>, // in type
  // 	EvalIRaz<u32,StdRng>,  // out type
  // > = test.test(&mut rng);
  let result: TestMResult<
  	EvalVec<Lang,StdRng>,
  	EvalVec<u32,StdRng>,
  > = test.test(&mut rng);

  println!("first inc parse time: {:?}", result.computes[1][1]);

}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ref_impl() {
		let prog = "23+4+4,23,54,123,5++".to_string();
		let toks = tokenize(prog);
		let vals = parse(toks);

		assert_eq!(vec![27, 4, 23, 182], vals);
	}
}