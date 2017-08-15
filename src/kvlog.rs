use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::HashMap;

use adapton::engine::*;
use trie2;

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
struct Hashval ( u64 );

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub struct Log<K,V> ( Option<Chunk<K, V>> );

type HashvalMask = (Hashval, usize);

#[derive(Clone,PartialEq,Eq,Debug)]
pub struct Chunk<K,V> {
    here: Vec<(K, V)>,
    there: JumpTable<K, V>,
    prev: Option<Art<Chunk<K, V>>>,
}

type JumpTable<K,V> = HashMap<usize,Option<Art<Chunk<K,V>>>> ;

impl<K:Hash,V:Hash> Hash for Chunk<K,V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.here.hash(state);
        self.prev.hash(state);        
        for (ref bits, ref opart) in self.there.iter() {
            bits.hash(state);
            opart.hash(state);
        }
    }
}

impl <K:'static+Hash+PartialEq+Eq+Clone+Debug,
      V:'static+Hash+PartialEq+Eq+Clone+Debug> Log<K,V> {

    pub fn emp() -> Log<K,V> { Log(None) }

    pub fn put(&mut self, key: K, val: V) {
        unimplemented!()
    }

    pub fn get(&mut self, key: K) -> Option<V> {
        unimplemented!()
    }

    pub fn into_trie(self) -> trie2::Trie<K,V> {
        unimplemented!()
    }
}
