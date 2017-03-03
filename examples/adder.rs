extern crate pmfp_collections;
extern crate adapton;
extern crate eval;
extern crate time;
extern crate rand;

use std::marker::PhantomData;
use time::Duration;
use rand::{Rng,StdRng,SeedableRng};
use eval::actions::{Creator,Computor,Testor,IncrementalEmpty,Folder};
use eval::primitives::{CreateEmpty,EditSeq,CompFold};
use eval::eval_vec::EvalVec;

#[derive(Debug,Clone,Copy,Hash,Eq,PartialEq)]
enum Token {
	Num(u32),
	OpPlus,
}

// reference tokenizer
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

struct Initializer<G:Rng>(IncrementalEmpty<G>);
struct Tokenizer<G:Rng>(IncrementalEmpty<G>);
struct Parser<G:Rng>(IncrementalEmpty<G>);
struct ParseTest<G:Rng> {
	to_parse: String,
	initializer: Initializer<G>,
	tokenizer: Tokenizer<G>,
	parser: Parser<G>,
}
struct ParseResult<C,T,N> {
	build_time: Duration,
	tokenize_time: Duration,
	parse_time: Duration,
	result: Vec<u32>,
	cstruct: PhantomData<C>,
	tstruct: PhantomData<T>,
	nstruct: PhantomData<N>,
}
impl<T,C,G>
Computor<T,C>
for Tokenizer<G> where
	T:CreateEmpty<G>+EditSeq<Token>+Clone,
	C:CompFold<char,Option<u32>,Fn((T,Option<u32>),&char)>,
	G:Rng
{
	fn compute(&mut self, data: &C, rng: &mut StdRng) -> T {
		let tokens:T = (self.0).create(rng).1;
		let tokenize = Folder::new((tokens,None),|(mut ts,part),c:&char|{
			if let Some(num) = c.to_digit(10) {
				let bigger = part.map_or(num,|p|p*10+num);
				(ts,Some(bigger))
			} else {
				if let Some(num) = part {
					ts = ts.push(Token::Num(num),rng).1;
				}
				match *c {
					'+' => {ts = ts.push(Token::OpPlus,rng).1;},
					',' => {},
					c => panic!("invalid char '{}'",c),
				}
				(ts,None)
			}
		});
		let (_duration,result) = tokenize.compute(data,rng);
		result
	}
}
impl<T,N:CreateEmpty<G>+EditSeq<u32>+Clone,G:Rng> Computor<N,T>
for Parser<G> {
	fn compute(&mut self, data: &T, rng: &mut StdRng) -> N {
		let nums:N = (self.0).create(rng).1;
		let parse = Folder::new(nums,|mut n,t|{
			match *t {
				Token::Num(a) => n.push(a,rng).1,
				Token::OpPlus => {
					let (_,b,n) = n.pop(rng);
					if let Some(b) = b {
						let (_,a,n) = n.pop(rng);
						if let Some(a) = a {
							n.push(a+b,rng).1
						} else {
							n.push(b,rng).1
						}
					} else { n }
				}
			}
		});
		let (_duration,result) = parse.compute(data,rng);
		result
	}
}

impl<C:CreateEmpty<G>+EditSeq<char>+Clone,T,N,G:Rng>
Testor<ParseResult<C,T,N>>
for ParseTest<G> {
	fn test(&mut self, rng: &mut StdRng) -> ParseResult<C,T,N> {
		let (_dur,cs):(Duration,C) = self.initializer.0.create(rng);
		for c in self.to_parse.chars() {
			cs = cs.push(c,rng).1
		}
		let ts = self.tokenizer.compute(&cs,rng);
		let ns = self.parser.compute(&ts,rng);
		ParseResult {
			build_time: Duration::zero(),
			tokenize_time: Duration::zero(),
			parse_time: Duration::zero(),
			result: ns.into_iter().collect(),
			cstruct: PhantomData,
			tstruct: PhantomData,
			nstruct: PhantomData,
		}
	}
}

fn main() {
	let mut parse = ParseTest {
		to_parse: String::from("4,5+8,7"),
		initializer: Initializer(IncrementalEmpty{
			unitgauge: 100,
			namegauge: 1,
			coord: StdRng::from_seed(&[0]),
		}),
		tokenizer: Tokenizer(IncrementalEmpty{
			unitgauge: 100,
			namegauge: 1,
			coord: StdRng::from_seed(&[0]),
		}),
		parser: Parser(IncrementalEmpty{
			unitgauge: 100,
			namegauge: 1,
			coord: StdRng::from_seed(&[0]),
		}),
	};

	let rng = StdRng::from_seed(&[0]);
	let result: ParseResult<EvalVec<char,StdRng>,EvalVec<Token,StdRng>,EvalVec<u32,StdRng>>
	= parse.test(&mut rng);

	println!("{:?}", result.result);

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