use inc_gauged_raz::*;
use std::hash::Hash;
use std::fmt::Debug;
use adapton::engine::name_of_usize;
use std::vec::Vec;
use std::collections::vec_deque::VecDeque;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

#[derive(Debug, Clone)]
struct SizedMap<V> where V: Clone + Debug + Eq + Hash + 'static {
	size: usize,
	gran: usize,
	map: RazTree<Option<V>>
}

pub trait FinMap<K, V> {
	//first usize: total size, second: granularity
	fn new(usize, usize) -> Self;
	
	fn size(Self) -> usize;
	
	fn gran(Self) -> usize;
	
	fn put(Self, K, V) -> Self;
	
	fn get(Self, K) -> Option<V>;
	
	fn contains(Self, K) -> bool;
	
	//Should this return a tuple: (Self, Option<V>)? got error: Self must have "Sized" trait
	fn del(Self, K) -> (Option<V>, Self);
}

impl<V> FinMap<usize, V> for SizedMap<V> where V: Clone + Debug + Eq + Hash {
	fn new(size: usize, gran: usize) -> Self {
		let mut store: Raz<Option<V>> = Raz::new();
		
		for x in 0..size {
			Raz::push_right(&mut store, None);
			if (x%gran) == 0 {
				Raz::archive_right(&mut store, x as u32, Some(name_of_usize(x)))
			}
		}
		
		SizedMap { size: size, gran: gran, map: Raz::unfocus(store) }
	}
	
	fn size(curr: Self) -> usize {
		curr.size
	}
	
	fn gran(curr: Self) -> usize {
		curr.gran
	}
	
	fn put(curr: Self, key: usize, val: V) -> Self {
		let mut seq_view = RazTree::focus(curr.map, key).unwrap();
		Raz::pop_right(&mut seq_view);
		Raz::push_right(&mut seq_view, Some(val));
		SizedMap { size: curr.size, gran: curr.gran, map: Raz::unfocus(seq_view) }
	}
	
	fn get(curr: Self, key: usize) -> Option<V> {
		let mut seq_view = RazTree::focus(curr.map, key).unwrap();
		Raz::peek_right(&mut seq_view).unwrap().clone().take()
	}
	
	fn contains(curr: Self, key: usize) -> bool {
		let mut seq_view = RazTree::focus(curr.map, key).unwrap();
		(*Raz::peek_right(&mut seq_view).unwrap()).is_some()
	}
	
	fn del(curr: Self, key: usize) -> (Option<V>, Self) {
		let mut seq_view = RazTree::focus(curr.map, key).unwrap();
		let ret = Raz::pop_right(&mut seq_view).unwrap();
		Raz::push_right(&mut seq_view, None);
		(ret, SizedMap { size: curr.size, gran: curr.gran, map: Raz::unfocus(seq_view) })
	}
}

//undirected graph
pub trait Graph<NdId, Data> {
	//usize params are for size/granularity pass to maps: should be exposed or no?
	fn new(usize, usize) -> Self;
	
	fn add_node(Self, NdId, Option<Data>) -> Self;
	
	fn del_node(Self, NdId) -> (Option<NdId>, Self);
	
	fn add_edge(Self, NdId, NdId) -> Self;
	
	fn del_edge(Self, NdId, NdId) -> Self;
	
	fn adjacents(Self, NdId) -> Option<Vec<NdId>>;
	
	fn get_data(Self, NdId) -> Option<Data>;
	
	fn bfs(Self, NdId) -> Self where Self : DirectedGraph<NdId, usize>;
	
	fn dfs(Self, NdId) -> Self where Self : DirectedGraph<NdId, usize>;
}

impl<T, Data> Graph<usize, Data> for T 
	where T: FinMap<usize, (Option<Data>, Vec<usize>)> + Clone
	{
	fn new(size: usize, gran: usize) -> Self {
		FinMap::new(size, gran)
	}
	
	fn add_node(curr: Self, id: usize, dt: Option<Data>) -> Self {
		FinMap::put(curr, id, (dt, vec!()))
	}
	
	fn del_node(curr: Self, id: usize) -> (Option<usize>, Self) {
		match FinMap::del(curr, id) {
			(None, new) => (None, new),
			(Some(_), new) => (Some(id), new)
		}
	}
	
	fn add_edge(curr: Self, id1: usize, id2: usize) -> Self {
		let mut ret = curr;
		if !FinMap::contains(ret.clone(), id1) {
			ret = Graph::add_node(ret.clone(), id1, None);
		}
		if !FinMap::contains(ret.clone(), id2) {
			ret = Graph::add_node(ret.clone(), id2, None);
		}
		let (k, mut adjs) = FinMap::get(ret.clone(), id1).unwrap();
		adjs.push(id2);
		ret = FinMap::put(ret, id1, (k, adjs));
		let(k, mut adjs2) = FinMap::get(ret.clone(), id2).unwrap();
		adjs2.push(id1);
		ret = FinMap::put(ret, id2, (k, adjs2));
		ret
	}
	
	fn del_edge(curr: Self, id1: usize, id2: usize) -> Self {
		let (k, mut adjs) = FinMap::get(curr.clone(), id1).unwrap();
		//is this an efficient/idiomatic way to do this?
		adjs.retain( |x: &usize| { x != &id2 } );
		let mut ret = FinMap::put(curr, id1, (k, adjs));
		let (k, mut adjs2) = FinMap::get(ret.clone(), id2).unwrap();
		adjs2.retain( |x: &usize| {x != &id1} );
		ret = FinMap::put(ret, id2, (k, adjs2));
		ret
	}
	
	fn adjacents(curr: Self, id: usize) -> Option<Vec<usize>> {
		match FinMap::get(curr, id) {
			Some((_, adjs)) => Some(adjs),
			None => None
		}
	}
	
	fn get_data(curr: Self, id: usize) -> Option<Data> {
		match FinMap::get(curr, id).unwrap() {
			(Some(d), _) => Some(d),
			(None, _) => None
		}
	}
	
	//Question: want to keep graph size available (for size of visited map), best way to do this?
	//action point: change return type to directed graph
	fn bfs(curr: Self, root: usize) -> Self where Self : DirectedGraph<usize, usize> {
		//setup
		let mut q : VecDeque<usize> = VecDeque::new();
		let mut v : SizedMap<bool> = FinMap::new(FinMap::size(curr.clone()), FinMap::gran(curr.clone()));
		let mut g : T = Graph::new(FinMap::size(curr.clone()), FinMap::gran(curr.clone()));
		
		q.push_back(root);
		g = Self::add_node(g, root, Some(0));
		v = FinMap::put(v, root, true);
		while !q.is_empty() {
			//get next element in queue
			//get c's level
			let c = q.pop_front().unwrap();
			let c_lev = Self::get_data(g.clone(), c).unwrap();
			let adjs = Self::adjacents(curr.clone(), c).unwrap();
			//iterate over nodes adjacent to c
			for n in adjs {
				//if n is not yet visited
				if !FinMap::contains(SizedMap { size: v.size, gran: v.gran, map: v.map.clone() }, n) {
					//this level is c's + 1
					g = Graph::add_node(g, n, Some(c_lev + 1));
					g = DirectedGraph::add_directed_edge(g, c, n);
					v = FinMap::put(v, n, true);
					q.push_back(n)
				}
			}
		}
		g
	}
	
	fn dfs(curr: Self, root: usize) -> Self where Self : DirectedGraph<usize, usize> {
		//setup
		let mut v : SizedMap<bool> = FinMap::new(FinMap::size(curr.clone()), FinMap::gran(curr.clone()));
		let mut s : VecDeque<usize> = VecDeque::new();
		let mut g : T = Graph::new(FinMap::size(curr.clone()), FinMap::gran(curr.clone()));
		
		//initialize DFS with the given root node
		s.push_back(root);
		g = Self::add_node(g, root, Some(0));
		v = FinMap::put(v, root, true);
		
		//begin loop
		while !s.is_empty() {
			let c = s.pop_back().unwrap();
			//if c hasn't yet been visited
			if !FinMap::contains(SizedMap{ size: v.size, gran: v.gran, map: v.map.clone() }, c) {
				//add c to the visited set
				v = FinMap::put(v, c, true);
				
				let adjs = DirectedGraph::successors(curr.clone(), c).unwrap();
				
				for n in adjs {
					s.push_back(n);
					g = DirectedGraph::add_directed_edge(g, c, n);
				}
			}
		}
		g
	}
}

pub trait DirectedGraph<NdId, Data> : Graph<NdId, Data> {
	fn add_directed_edge(Self, NdId, NdId) -> Self;
	
	fn successors(Self, NdId) -> Option<Vec<NdId>>;
}

impl<T, Data> DirectedGraph<usize, Data> for T 
	where T: FinMap<usize, (Option<Data>, Vec<usize>)> + Clone
	{
	fn add_directed_edge(curr: Self, src: usize, dst: usize) -> Self {
		let mut ret = curr;
		if !FinMap::contains(ret.clone(), src) {
			ret = Graph::add_node(ret.clone(), src, None);
		}
		if !FinMap::contains(ret.clone(), dst) {
			ret = Graph::add_node(ret.clone(), dst, None);
		}
		let (k, mut adjs) = FinMap::get(ret.clone(), src).unwrap();
		adjs.push(dst);
		FinMap::put(ret, src, (k, adjs))
	}
	
	fn successors(curr: Self, id: usize) -> Option<Vec<usize>> {
		Graph::adjacents(curr, id)
	}
}
	

fn dgraph_from_col<T:DirectedGraph<usize, usize> + Clone + FinMap<usize, (Option<usize>, Vec<usize>)>>(path: String) -> T {
	let f = File::open(path).unwrap();
	let reader = BufReader::new(f);
	let lines: Vec<_> = reader.lines().collect();
	
	let mut g: T = Graph::new(10000, 10000);
	
	for l in lines {
		let line = match l {
			Ok(inner) => inner,
			Err(_) => break
		};
		let mut words = line.split_whitespace();
		
		if words.next() == Some("e") {
			let v1: usize = match u32::from_str_radix(words.next().unwrap(), 10) {
				Ok(k) => k as usize,
				Err(_) => break
			};
			let v2: usize = match u32::from_str_radix(words.next().unwrap(), 10) {
				Ok(k) => k as usize,
				Err(_) => break
			};
			
			g = DirectedGraph::add_directed_edge(g.clone(), v1, v2);
		}
	}
	g
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
  fn test_fin_map() {
  	let mut dt: SizedMap<usize> = FinMap::new(100, 10);
  	dt = FinMap::put(dt, 10, 10);
  	dt = FinMap::put(dt, 11, 11);
  	dt = FinMap::put(dt, 12, 12);
  	assert_eq!(Some(12), FinMap::get(dt.clone(), 12));
  	let dt =
	  	match FinMap::del(dt, 11) {
	  		(_, v) => v
	  	};
  	assert_eq!(None, FinMap::get(dt, 11));
  }
  
  #[test]
  fn test_graph() {
  	let mut dt: SizedMap<(Option<usize>, Vec<usize>)> = Graph::new(100, 10);
  	dt = Graph::add_node(dt, 1, Some(1));
  	dt = Graph::add_node(dt, 2, Some(2));
  	dt = Graph::add_node(dt, 3, Some(3));
  	dt = Graph::add_edge(dt, 1, 2);
  	dt = Graph::add_edge(dt, 2, 3);
  	dt = Graph::add_edge(dt, 3, 1);
  	assert_eq!(Some(vec!(2, 3)), Graph::adjacents(dt.clone(), 1));
  	dt = Graph::del_edge(dt, 1, 2);
  	assert_eq!(Some(vec!(3)), Graph::adjacents(dt.clone(), 1))
  }
  
  #[test]
  fn test_bfs() {
  	let mut dt: SizedMap<(Option<usize>, Vec<usize>)> = Graph::new(100, 10);
  	dt = Graph::add_node(dt, 1, Some(1));
  	dt = Graph::add_node(dt, 2, Some(2));
  	dt = Graph::add_node(dt, 3, Some(3));
  	dt = Graph::add_node(dt, 4, Some(4));
  	dt = Graph::add_edge(dt, 1, 2);
  	dt = Graph::add_edge(dt, 1, 3);
  	dt = Graph::add_edge(dt, 2, 4);
  	dt = Graph::add_edge(dt, 3, 4); //this is the basic diamond graph
  	
  	let bfs_tree = Graph::bfs(dt, 1);
  	
  	assert_eq!(Some(vec!(2, 3)), DirectedGraph::successors(bfs_tree.clone(), 1));
  	assert_eq!(Some(vec!(4)), DirectedGraph::successors(bfs_tree.clone(), 2));
  	assert_eq!(Some(vec!()), DirectedGraph::successors(bfs_tree.clone(), 3));
  	assert_eq!(Some(vec!()), DirectedGraph::successors(bfs_tree.clone(), 4));
  	assert_eq!(Some(0), Graph::get_data(bfs_tree.clone(), 1));
  	assert_eq!(Some(1), Graph::get_data(bfs_tree.clone(), 2));
  	assert_eq!(Some(1), Graph::get_data(bfs_tree.clone(), 3));
  	assert_eq!(Some(2), Graph::get_data(bfs_tree.clone(), 4));
  }
  
  #[test]
  fn test_simple_parser() {
  	use std::time::Instant;
  	
  	let path: String = "fpsol2_graph.txt".to_owned();
  	let dt: SizedMap<(Option<usize>, Vec<usize>)> = dgraph_from_col(path);
  	
  	let start = Instant::now();
  	
  	let bfs_res = Graph::bfs(dt.clone(), 1);
  	
  	let end = start.elapsed();
  	
  	println!("bfs took {} nanoseconds", end.subsec_nanos());
  	
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 30);
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 31);
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 32);
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 33);
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 34);
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 35);
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 36);
  	
  	let start2 = Instant::now();
  	
  	let bfs_res2 = Graph::bfs(dt.clone(), 1);
  	
  	let end2 = start2.elapsed();
  	
  	println!("bfs2 took {} nanoseconds", end2.subsec_nanos());
  	
  	assert_eq!(true, FinMap::contains(dt.clone(), 416));
  	assert_eq!(false, FinMap::contains(dt.clone(), 450));
  }
  
  #[test]
  #[should_panic]
  fn test_gran() {
  	let dt: SizedMap<(Option<usize>, Vec<usize>)> = Graph::new(100, 10);
  	
  	let dt = DirectedGraph::add_directed_edge(dt, 1, 2);
  	let dt = DirectedGraph::add_directed_edge(dt, 10, 11);
  	let dt = DirectedGraph::add_directed_edge(dt, 20, 21);
  	let dt = DirectedGraph::add_directed_edge(dt, 30, 39);
  }
}