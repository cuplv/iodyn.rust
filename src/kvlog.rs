use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use adapton::engine::*;
use trie2;

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
struct Hashval ( u64 );

pub type Log<K,V> = Option<Chunk<K, V>>;

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
enum Trie<K,V> {
    Bin(Box<Trie<K,V>>, Box<Trie<K,V>>),
    Mapping(HashvalMask, Mapping<K,V>)
}

type HashvalMask = (Hashval, usize);

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
enum Mapping<K,V> {
    Empty,
    Here,
    There(Art<Chunk<K, V>>),
}

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub struct Chunk<K,V> {
    here: Vec<(K, V)>,
    there: Option<Trie<K, V>>,
    prev: Option<Art<Chunk<K,V>>>,
}

pub fn emp
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    () -> Log<K,V> { None }

fn compute_trie
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (log:Log<K,V>) -> Log<K,V> {
        match log {
            None => None,
            Some(chunk) => {
                // compute 'there' trie for the key-value pairs that
                // are 'here' in the chunk.
                unimplemented!()
            }
        }
    }

pub fn archive
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (an:Option<Name>, log:Log<K,V>) -> Log<K,V> {
        match an {
            None => log,
            Some(n) => {
                let log = compute_trie(log);
                Some(Chunk{
                    here: Vec::new(),
                    there: None,
                    prev: match log {
                        None => None,
                        Some(c) => Some(cell(n, c))
                    }
                })
            }
        }
    }

pub fn put
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (an:Option<Name>, log:Log<K,V>, k:K, v:V) -> Log<K,V> {
        let mut log = archive(an, log);
        match log {
            None =>
                return Some(Chunk{
                    here: vec![(k,v)],
                    there: None,
                    prev: None,
                }),
            Some(ref mut c) => {
                c.here.push((k,v));
            }
        };
        return log
    }

pub fn chunk_find
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (c:&Chunk<K,V>, k:&K) -> Option<V> {
        for &(ref ck, ref cv) in c.here.iter() /* FIXME: reverse! */ {
            if k == ck { return Some(cv.clone()); }
        };
        // Do a look-up in the Trie 'there', to find the next
        // Chunk to search with a linear scan (as above).
        unimplemented!()
    }


pub fn get
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (an:Option<Name>, mut log:Log<K,V>, k:&K) -> (Log<K,V>, Option<V>) {
        match log {
            None => (None, None),
            Some(c) => {
                let ret = chunk_find(&c, &k);
                // if value is found, record it 'here', otherwise, record nothing in log.
                match ret {
                    None =>        { log = Some(c) },
                    Some(ref v) => { log = put(an, Some(c), k.clone(), v.clone()) }
                };
                (log, ret)
            }
        }
    }

pub fn into_trie
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    (trie:Log<K,V>) -> trie2::Trie<K,V> {
        unimplemented!()
    }
