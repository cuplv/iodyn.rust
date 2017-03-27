use inc_gauged_raz::*;
use std::hash::Hash;
use std::fmt::Debug;
use adapton::engine::name_of_usize;
use std::vec::Vec;

trait FinMap<K, V> {
	//first usize: total size, second: granularity
	fn new(usize, usize) -> Self;
	
	fn put(Self, K, V) -> Self;
	
	fn get(Self, K) -> Option<V>;
	
	//Should this return a tuple: (Self, Option<V>)? got error: Self must have "Sized" trait
	fn del(Self, K) -> (Option<V>, Self);
}

impl<V> FinMap<usize, V> for RazTree<Option<V>> where V: Clone + Debug + Eq + Hash {
	fn new(size: usize, gran: usize) -> Self {
		let mut store: Raz<Option<V>> = Raz::new();
		
		for x in 0..size {
			Raz::push_right(&mut store, None);
			if (x%gran) == 0 {
				Raz::archive_right(&mut store, x as u32, Some(name_of_usize(x)))
			}
		}
		
		Raz::unfocus(store)
	}
	
	fn put(curr: Self, key: usize, val: V) -> Self {
		let mut seq_view = RazTree::focus(curr, key).unwrap();
		Raz::pop_right(&mut seq_view);
		Raz::push_right(&mut seq_view, Some(val));
		Raz::unfocus(seq_view)
	}
	
	fn get(curr: Self, key: usize) -> Option<V> {
		let mut seq_view = RazTree::focus(curr, key).unwrap();
		Raz::peek_right(&mut seq_view).unwrap().clone().take()
	}
	
	fn del(curr: Self, key: usize) -> (Option<V>, Self) {
		let mut seq_view = RazTree::focus(curr, key).unwrap();
		let ret = Raz::pop_right(&mut seq_view).unwrap();
		Raz::push_right(&mut seq_view, None);
		(ret, Raz::unfocus(seq_view))
	}
}

//undirected graph
trait Graph<NdId, NdData> {
	//usize params are for size/granularity pass to maps: should be exposed or no?
	fn new(usize, usize) -> Self;
	
	fn add_node(Self, NdId, NdData) -> Self;
	
	fn del_node(Self, NdId) -> (Option<NdId>, Self);
	
	//semantics for add/del_edge on not existing node?
	fn add_edge(Self, NdId, NdId) -> Self;
	
	//tuple return?
	fn del_edge(Self, NdId, NdId) -> Self;
	
	fn adjacents(Self, NdId) -> Option<Vec<NdId>>;
}

impl<T, Data> Graph<usize, (Data, Vec<usize>)> for T 
	where T: FinMap<usize, (Data, Vec<usize>)> + Clone + Copy
	{
	fn new(size: usize, gran: usize) -> Self {
		FinMap::new(size, gran)
	}
	
	fn add_node(curr: Self, id: usize, dt: (Data, Vec<usize>)) -> Self {
		FinMap::put(curr, id, dt)
	}
	
	fn del_node(curr: Self, id: usize) -> (Option<usize>, Self) {
		match FinMap::del(curr, id) {
			(None, _) => (None, curr),
			(Some(_), _) => (Some(id), curr)
		}
	}
	
	//Currently assumes that both nodes exist. Semantics undefined if nodes don't exist.
	//These changes persist into the Map, right?
	fn add_edge(curr: Self, id1: usize, id2: usize) -> Self {
		let (_, mut adjs) = FinMap::get(curr, id1).unwrap();
		adjs.push(id2);
		let(_, mut adjs2) = FinMap::get(curr, id2).unwrap();
		adjs2.push(id1);
		curr
	}
	
	fn del_edge(curr: Self, id1: usize, id2: usize) -> Self {
		let (_, mut adjs) = FinMap::get(curr, id1).unwrap();
		//is this an efficient/idiomatic way to do this?
		adjs.retain( |x: &usize| { x != &id2 } );
		let (_, mut adjs2) = FinMap::get(curr, id2).unwrap();
		adjs2.retain( |x: &usize| {x != &id1} );
		curr
	}
	
	fn adjacents(curr: Self, id: usize) -> Option<Vec<usize>> {
		match FinMap::get(curr, id) {
			Some((_, adjs)) => Some(adjs),
			None => None
		}
	}
}