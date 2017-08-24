/*
Look into Datalog frontends (Doop) and see if they have an IR to plug into?
Look at Soot in andersen's analysis (Spark?)
Look at cil as well. Ben hardekopf?
Find systematization of andersen's rule for Java
Do pre-processing hashmaps for algorithm?
*/

extern crate iodyn;

use iodyn::finite_map::*;
use std::rc::Rc;
use std::collections::vec_deque::VecDeque;
use std::time::Instant;

/*
Rules for Andersen's analysis:
0. p := &q ==> p -> q
1. p := q && q -> r ==> p -> r
2. p := *q && q -> r && r -> s ==> p -> s
3. *p := q && p -> r && q -> s ==> r -> s
*/

/*
AndersenRule corresponds to a rule from andersens analysis to be processed by the algorithm,
the edge (previously added to the graph) that triggered it, and the id of the corresponding statement.
Ex: 
If q -> r is added to the graph and p := *q is in the bag (rule 2), the AndersenRule added to the queue is:
stmtl = p, stmtr = q, edgel = q, edger = r, rulenum = 2
N is the type of the node from the graph
*/
pub struct AndersenRule<N> where N : Eq {
	stmtl: N,
	stmtr: N,
	edgel: N,
	edger: N,
	rulenum: usize,
}

#[derive(Debug,Clone)]
pub struct CStatement<N> where N : Eq + Clone {
	left: N,
	right: N,
	num: usize, //numbering follows rules above
}

fn andersen<N:Eq+Clone+std::fmt::Display,G:DirectedGraph<N,usize>+Clone>(stmts: Vec<CStatement<N>>) -> G {
	//based on an edge and a list of stmts, returns a list of AndersenRules to be checked
	fn gen_cand_rules<N:Eq+Clone>(left: N, right: N, stmts: Vec<CStatement<N>>) -> VecDeque<AndersenRule<N>> {
		let mut ret: VecDeque<AndersenRule<N>> = VecDeque::new();
		
		//rule 1: p := q && q -> r ==> p -> r
		let mut r1stmts = stmts.clone();
		r1stmts.retain(|x:&CStatement<N>| {x.num == 1 && x.right == left});
		for r in r1stmts {
			ret.push_back(AndersenRule{stmtl: r.left, stmtr: r.right, 
									   edgel: left.clone(), edger: right.clone(), rulenum: 1});
		}
		
		//rule 2: p := *q && q -> r && r -> s ==> p -> s
		let mut r2stmts = stmts.clone();
		r2stmts.retain(|x:&CStatement<N>| {x.num == 2});
		for r in r2stmts {
			ret.push_back(AndersenRule{stmtl: r.left, stmtr: r.right,
									   edgel: left.clone(), edger: right.clone(), rulenum: 2});
		}
		
		//rule 3: *p := q && p -> r && q -> s ==> r -> s
		let mut r3stmts = stmts.clone();
		r3stmts.retain(|x:&CStatement<N>| {x.num == 3 && (x.left == left || x.right == left)});
		for r in r3stmts {
			ret.push_back(AndersenRule{stmtl: r.left, stmtr: r.right,
									   edgel: left.clone(), edger: right.clone(), rulenum: 3});
		}
		
		ret
	}
	
	//based on an AndersenRule and an existing graph, outputs modified graph (with added edges) if necessary
	fn chkapply<N:Eq+Clone,G:DirectedGraph<N,usize>+Clone>(g: G, rule: AndersenRule<N>) -> (G, Vec<(N, N)>) {
		let mut retedges: Vec<(N, N)> = vec!();
		match rule.rulenum {
			//rule 1: p := q && q -> r ==> p -> r
			1 => {
				if !Graph::has_edge(g.clone(), rule.stmtl.clone(), rule.edger.clone()) {
					retedges.push((rule.stmtl.clone(), rule.edger.clone()));
					(DirectedGraph::add_directed_edge(g, rule.stmtl, rule.edger), retedges)
				} else {(g, retedges)}
			},
			//rule 2: p := *q && q -> r && r -> s ==> p -> s
			2 => {
				if rule.stmtr == rule.edgel {
					//edge is q -> r. Find each r -> s, and add p -> s
					let nexts: Vec<N> = Graph::adjacents(g.clone(), rule.edger);
					let mut ret = g.clone();
					for s in nexts {
						if !Graph::has_edge(g.clone(), rule.stmtl.clone(), s.clone()) {
							retedges.push((rule.stmtl.clone(), s.clone()));
							ret = DirectedGraph::add_directed_edge(ret, rule.stmtl.clone(), s);
						}
					}
					(ret, retedges)
				} else {
					//edge is r -> s. If q -> r exists, add p -> s
					if Graph::has_edge(g.clone(), rule.stmtr, rule.edgel) {
						if !Graph::has_edge(g.clone(), rule.stmtl.clone(), rule.edger.clone()) {
							retedges.push((rule.stmtl.clone(), rule.edger.clone()));
							(DirectedGraph::add_directed_edge(g.clone(), rule.stmtl, rule.edger), retedges)
						} else { (g, retedges) }
					} else { (g, retedges) }
				}
			},
			//rule 3: *p := q && p -> r && q -> s ==> r -> s
			3 => {
				if rule.stmtl == rule.edgel {
					//edge is p -> r. Find each q -> s, and add r -> s
					let nexts: Vec<N> = Graph::adjacents(g.clone(), rule.stmtr);
					let mut ret = g.clone();
					for s in nexts {
						if !Graph::has_edge(g.clone(), rule.edger.clone(), s.clone()) {
							retedges.push((rule.edger.clone(), s.clone()));
							ret = DirectedGraph::add_directed_edge(ret, rule.edger.clone(), s);
						}
					}
					(ret, retedges)
				} else if rule.stmtr == rule.edgel {
					//edge is q -> s. Find each p -> r, and add r -> s
					let nexts: Vec<N> = Graph::adjacents(g.clone(), rule.stmtl);
					let mut ret = g.clone();
					for r in nexts {
						if !Graph::has_edge(g.clone(), r.clone(), rule.edger.clone()) {
							retedges.push((r.clone(), rule.edger.clone()));
							ret = DirectedGraph::add_directed_edge(ret, r, rule.edger.clone());
						}
					}
					(ret, retedges)
				} else { panic!("bad rule of type 3 in chkapply"); }
			},
			_ => panic!("bad rulenum found its way to chkapply")
		}
	}
	
	let mut g: G = Graph::new(100, 10);
	let mut q: VecDeque<AndersenRule<N>> = VecDeque::new();
	
	//preprocess: initialize the graph by adding the base "rule 0" edges
	let mut roots = stmts.clone();
	roots.retain(|x:&CStatement<N>| {x.num == 0});
	for r in roots {
		g = Graph::add_node(g, r.left.clone(), None);
		g = Graph::add_node(g, r.right.clone(), None);
		g = DirectedGraph::add_directed_edge(g, r.left.clone(), r.right.clone());
		q.append(&mut gen_cand_rules(r.left.clone(), r.right.clone(), stmts.clone()));
	}
	
	fn process_queue<N:Eq+Clone+std::fmt::Display,G:DirectedGraph<N,usize>+Clone>((mut q, g, stmts):(VecDeque<AndersenRule<N>>, G, Vec<CStatement<N>>)) -> 
		Result<(VecDeque<AndersenRule<N>>, G, Vec<CStatement<N>>), G> {
			match q.pop_back() {
				Some(r) => {
					//println!("stmtl: {}, stmtr: {}, edgel: {}, edger: {}, rulenum: {}", r.stmtl, r.stmtr, r.edgel, r.edger, r.rulenum);
					let (g, edges) = chkapply(g, r);
					for (l, r) in edges {
						q.append(&mut gen_cand_rules(l, r, stmts.clone()));
					}
					Ok((q, g, stmts))
				},
				None => Err(g)
			}
		}
	
	unfold_simple((q, g, stmts), Rc::new(process_queue))
}

fn main () {
  use std::thread;
  let child =
    thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}

use std::io::{self,BufRead};
extern crate rand;
use rand::{thread_rng, Rng};
fn main2() {
	println!("Input number of variables for this example as an int: ");
	
	let mut line = String::new();
    let stdin = io::stdin();
    stdin.lock().read_line(&mut line).unwrap();
    let num_vars = line.trim().parse::<usize>().unwrap();
    
    //generate all distinct CStatements possible from num_vars variables
    let mut all_stmts: Vec<CStatement<usize>> = vec!();
    for i in 1..num_vars+1 {
    	for j in 1..num_vars+1 {
    		for k in 0..4 {
    			if i != j {
	    			all_stmts.push(CStatement{left: i, right: j, num: k});
    			}
    		}
    	}
    }
    
    println!("all_stmts size: {}", all_stmts.len());
    
    let mut stmts: Vec<CStatement<usize>> = vec!();
    let mut rng = thread_rng();
    for i in 1..num_vars.pow(2)+1 {
    	rng.shuffle(&mut all_stmts);
    	stmts.push(all_stmts.pop().expect("popped from empty vec"));
    }
    
    println!("statement list size: {}", stmts.len());
    println!("statment list: {:?}", stmts.clone());
    
    let mut dt: SizedMap<(Option<usize>, Vec<usize>)>;
    let start = Instant::now();
    dt = andersen(stmts.clone());
    println!("time elapsed using Raz: {} nanoseconds", start.elapsed().subsec_nanos());
    
    let mut dt: SizedHashMap<usize, (Option<usize>, Vec<usize>)>;
    let start = Instant::now();
    dt = andersen(stmts.clone());
    println!("time elapsed using Rust Hashmap: {} nanoseconds", start.elapsed().subsec_nanos());
	
    /*let mut dt: SizedMap<(Option<usize>, Vec<usize>)>;
    let stmts: Vec<CStatement<usize>> = vec!(CStatement { left: 2, right: 1, num: 0 }, 
									    	 CStatement { left: 2, right: 1, num: 1 }, 
									    	 CStatement { left: 1, right: 2, num: 1 }, 
									    	 CStatement { left: 1, right: 2, num: 2 });
    dt = andersen(stmts);*/
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn basic_statements() {
		let mut dt: SizedMap<(Option<usize>, Vec<usize>)>;
		let mut stmts = vec!(CStatement{left: 1, right: 0, num: 0});
		stmts.push(CStatement{left: 2, right: 1, num: 1});
		stmts.push(CStatement{left: 3, right: 2, num: 2});
		stmts.push(CStatement{left: 10, right: 9, num: 0});
		stmts.push(CStatement{left: 1, right: 10, num: 3});
		let start = Instant::now();
		dt = andersen(stmts.clone());
		println!("basic test time with Raz: {} nanoseconds", start.elapsed().subsec_nanos());
		assert_eq!(vec!(0), Graph::adjacents(dt.clone(), 1));
		assert_eq!(vec!(0), Graph::adjacents(dt.clone(), 2));
		assert_eq!(vec!(9), Graph::adjacents(dt.clone(), 3));
		assert_eq!(vec!(9), Graph::adjacents(dt.clone(), 0));
		
		println!("executed basic test");
		
		let mut dt: SizedHashMap<usize, (Option<usize>, Vec<usize>)>;
		let start = Instant::now();
		dt = andersen(stmts.clone());
		println!("basic test time with Rust Hashmap: {} nanoseconds", start.elapsed().subsec_nanos());
		assert_eq!(vec!(0), Graph::adjacents(dt.clone(), 1));
		assert_eq!(vec!(0), Graph::adjacents(dt.clone(), 2));
		assert_eq!(vec!(9), Graph::adjacents(dt.clone(), 3));
		assert_eq!(vec!(9), Graph::adjacents(dt.clone(), 0));
		
		println!("executed basic test with Hashmap");
	}
}