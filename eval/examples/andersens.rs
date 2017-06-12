extern crate iodyn;

use iodyn::finite_map::*;
use std::collections::vec_deque::VecDeque;

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
If q -> r is added to the graph and p := *q is in the bag (rule 3), the AndersenRule added to the queue is:
stmtl = p, stmtr = q, edgel = q, edger = r, rulenum = 3
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

fn andersen<N:Eq+Clone,G:DirectedGraph<N,usize>>(stmts: Vec<CStatement<N>>) -> G {
	//based on an edge and a list of stmts, returns a list of AndersenRules to be checked
	fn genCandRules<N:Eq+Clone>(left: N, right: N, stmts: Vec<CStatement<N>>) -> Vec<AndersenRule<N>> {
		let mut ret: Vec<AndersenRule<N>> = vec!();
		
		//rule 1: p := q && q -> r ==> p -> r
		let mut r1stmts = stmts.clone();
		r1stmts.retain(|x:&CStatement<N>| {x.num == 1 && x.right == left});
		for r in r1stmts {
			ret.push(AndersenRule{stmtl: r.left, stmtr: r.right, 
								  edgel: left.clone(), edger: right.clone(), rulenum: 1});
		}
		
		//rule 2: p := *q && q -> r && r -> s ==> p -> s
		let mut r2stmts = stmts.clone();
		r2stmts.retain(|x:&CStatement<N>| {x.num == 2});
		for r in r2stmts {
			ret.push(AndersenRule{stmtl: r.left, stmtr: r.right,
								  edgel: left.clone(), edger: right.clone(), rulenum: 2});
		}
		
		//rule 3: *p := q && p -> r && q -> s ==> r -> s
		let mut r3stmts = stmts.clone();
		r3stmts.retain(|x:&CStatement<N>| {x.num == 3 && (x.left == left || x.right == left)});
		for r in r3stmts {
			ret.push(AndersenRule{stmtl: r.left, stmtr: r.right,
								  edgel: left.clone(), edger: right.clone(), rulenum: 3});
		}
		
		ret
	}
	
	//based on an AndersenRule and an existing graph, outputs modified graph if necessary
	fn chkapply<N:Eq+Clone,G:DirectedGraph<N,usize>+Clone>(g: G, rule: AndersenRule<N>) -> G {
		match rule.rulenum {
			//rule 1: p := q && q -> r ==> p -> r
			1 => DirectedGraph::add_directed_edge(g, rule.stmtl, rule.edger),
			//rule 2: p := *q && q -> r && r -> s ==> p -> s
			2 => {
				if rule.stmtr == rule.edgel {
					//edge is q -> r. Find each r -> s, and add p -> s
					let nexts: Vec<N> = Graph::adjacents(g.clone(), rule.edger);
					let mut ret = g.clone();
					for s in nexts {
						ret = DirectedGraph::add_directed_edge(ret, rule.stmtl.clone(), s);
					}
					ret
				} else {
					//edge is r -> s. If q -> r exists, add p -> s
					if Graph::has_edge(g.clone(), rule.stmtr, rule.edgel) {
						DirectedGraph::add_directed_edge(g.clone(), rule.stmtl, rule.edger)
					} else { g }
				}
			},
			//rule 3: *p := q && p -> r && q -> s ==> r -> s
			3 => {
				if rule.stmtl == rule.edgel {
					//edge is p -> r. Find each q -> s, and add r -> s
					let nexts: Vec<N> = Graph::adjacents(g.clone(), rule.stmtr);
					let mut ret = g.clone();
					for s in nexts {
						ret = DirectedGraph::add_directed_edge(ret, rule.edger.clone(), s);
					}
					ret
				} else if rule.stmtr == rule.edgel {
					//edge is q -> s. Find each p -> r, and add r -> s
					let nexts: Vec<N> = Graph::adjacents(g.clone(), rule.stmtl);
					let mut ret = g.clone();
					for r in nexts {
						ret = DirectedGraph::add_directed_edge(ret, r, rule.edger.clone());
					}
					ret
				} else { panic!("bad rule of type 3 in chkapply"); }
			},
			_ => panic!("bad rulenum found its way to chkapply")
		}
	}
	
	let g: G = Graph::new(stmts.len(), (stmts.len()/10)+1);
	let q: VecDeque<AndersenRule<N>> = VecDeque::new();
	
	//preprocess: initialize the graph by adding the base "rule 0" edges
	let mut roots = stmts.clone();
	roots.retain(|x:&CStatement<N>| {x.num == 0});
	panic!("stubbed");
}

fn main() {
	println!("it's the main!");
}