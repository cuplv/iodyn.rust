use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use adapton::engine::*;
use trie2;

struct Hashval ( u64 );

pub enum Log<K,V> {
    Empty,
    Chunks(Chunk<K, V>)
}

enum Trie<K,V> {
    Bin(Box<Trie<K,V>>, Box<Trie<K,V>>),
    Mapping(HashvalMask, Mapping<K,V>)
}

type HashvalMask = (Hashval, usize);

enum Mapping<K,V> {
    Empty, 
    Here, 
    There(Art<Chunk<K, V>>),
}

pub struct Chunk<K,V> {
    here: Vec<(K, V)>,
    there: Trie<K, V>,
    prev: Art<Chunk<K, V>>
}

pub fn emp
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    () -> Log<K,V> { 
        Log::Empty
    }

pub fn put
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (an:Option<Name>, log:Log<K,V>, k:K, v:V) -> Log<K,V> {
        unimplemented!()
    }

pub fn get
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (an:Option<Name>, log:Log<K,V>, k:K) -> (Log<K,V>, Option<V>) {
        unimplemented!()
    }

pub fn into_trie
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (trie:Log<K,V>) -> trie2::Trie<K,V> {
        unimplemented!()
    }
