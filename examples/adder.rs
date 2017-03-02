extern crate pmfp_collections;
extern crate adapton;
extern crate eval;

#[derive(Debug,Clone)]
enum Token {
	Num(u32),
	OpPlus,
}

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

fn main() {
	let prog = "23+4+4,23,54,123,5++".to_string();
	let toks = tokenize(prog);
	println!("{:?}", toks);
	let vals = parse(toks);
	println!("{:?}", vals);
}