//! Binary Hash Tries, for representing sets and finite maps.
//!
//! Suitable for the Archivist role in Adapton.
//!
// Matthew Hammer <Matthew.Hammer@Colorado.edu>

//use std::rc::Rc;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash,Hasher};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use adapton::engine::{cell,name_fork,force,Art,Name};

fn my_hash<T>(obj: T) -> u64
  where T: Hash
{
  let mut hasher = DefaultHasher::new();
  obj.hash(&mut hasher);
  hasher.finish()
}

/// A hash value -- We define a custom Debug impl for this type.
#[derive(Clone,Hash,Eq,PartialEq)]
pub struct HashVal(usize);

impl fmt::Debug for HashVal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

/// TODO-Someday: Do this more efficiently
fn make_mask(len:usize) -> usize {
    assert!(len > 0);
    let mut mask = 0;
    for _ in 0..len {
        mask <<= 1;
        mask |= 0x1;
    }
    return mask;
}

#[derive(PartialEq,Eq,Clone,Debug,Hash)]
struct Bits {bits:u32, len:u32}

#[derive(PartialEq,Eq,Clone,Debug, Hash)]
pub enum Trie<K:'static+Hash+Eq+Clone+Debug,
              V:'static+Hash+Eq+Clone+Debug>
{
    Empty,
    Leaf(TrieLeaf<K,V>),
    Bin(TrieBin<K,V>),
}
#[derive(Eq,Clone,Debug)]
pub struct TrieLeaf<K:'static+Hash+Eq+Clone+Debug,
                    V:'static+Hash+Eq+Clone+Debug> {
    hash:HashVal,
    map:HashMap<K,V>,
}
#[derive(Hash,PartialEq,Eq,Clone,Debug)]
pub struct TrieBin<K:'static+Hash+Eq+Clone+Debug,
                   V:'static+Hash+Eq+Clone+Debug> {
    bits:   Bits,
    name:   Name,
    left:   Art<Trie<K,V>>,
    right:  Art<Trie<K,V>>,
}

impl<K:'static+Hash+Eq+Clone+Debug,
     V:'static+Hash+Eq+Clone+Debug>
    Hash for TrieLeaf<K,V> 
{
    fn hash<H:Hasher>(&self, h:&mut H) {
        self.hash.hash(h)
    }
}
impl<K:'static+Hash+Eq+Clone+Debug,
     V:'static+Hash+Eq+Clone+Debug>
    PartialEq for TrieLeaf<K,V> 
{
    fn eq(&self, other:&Self) -> bool {
        self.hash == other.hash
    }
}

impl<K:'static+Hash+Eq+Clone+Debug,
     V:'static+Hash+Eq+Clone+Debug> Trie<K,V> {

    pub fn find (t: &Self, h:HashVal, k:&K) -> Option<V> {
        match t {
            &Trie::Empty => None,
            &Trie::Leaf(ref l) => match l.map.get(k) { Some(v) => Some(v.clone()), None => None },
            &Trie::Bin(ref b) => {
                if h.0 & 1 == 0 {
                    Self::find(&get!(b.left), HashVal(h.0 >> 1), k)
                } else { 
                    Self::find(&get!(b.right), HashVal(h.0 >> 1), k)
                }
            }
        }
    }

    fn hash_map (map: &HashMap<K,V>) -> HashVal {
        let mut state = DefaultHasher::new();
        for (k,v) in map.iter() {
            k.hash(&mut state);
            v.hash(&mut state);
        };
        HashVal(state.finish() as usize)
    }

    fn split_map (map: HashMap<K,V>, bits:&Bits,
                  mut map0:HashMap<K,V>, 
                  mut map1:HashMap<K,V>)
                  -> (HashMap<K,V>, HashMap<K,V>) 
    {
        let mask : u64 = make_mask(bits.len as usize) as u64;
        for (k,v) in map.into_iter() {
            let k_hash = my_hash(&k);
            assert!(mask & k_hash == bits.bits as u64);
            if 0 == k_hash & (1 << (bits.len + 1)) {
                map0.insert(k, v);
            } else {
                map1.insert(k, v);
            }
        };
        (map0, map1)
    }

    pub fn empty () -> Self { 
        Trie::Empty 
    }

    pub fn from_hashmap(hm:HashMap<K,V>) -> Self { 
        Trie::Leaf(TrieLeaf{hash:Self::hash_map(&hm), map:hm})
    }

    pub fn union (lt: Self, rt: Self, n:Name) -> Self {
        Self::union_rec(lt, rt, Bits{len:0, bits:0}, n)
    }

    // Questions:
    //
    // 1. When do we decide to "merge" leaves?  Based on where we have
    // or don't have names/levels?
    //
    // 2. Is there a way to shorten the code below? There seems to be
    // patterns that I'm not exploiting.

    fn union_rec (lt: Self, rt: Self, bits:Bits, n:Name) -> Self {
        match (lt, rt) {
            (Trie::Empty, rt) => rt,
            (lt, Trie::Empty) => lt,
            (Trie::Leaf(l), Trie::Leaf(r)) => {
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (l0, l1) = Self::split_map(l.map, &bits, e0, e1);
                let (r0, r1) = Self::split_map(r.map, &bits, l0, l1);
                let (n1, n2) = name_fork(n.clone());
                let lf1 = TrieLeaf{hash:Self::hash_map(&r0), map:r0};
                let lf2 = TrieLeaf{hash:Self::hash_map(&r1), map:r1};
                let r0 = cell(n1, Trie::Leaf(lf1));
                let r1 = cell(n2, Trie::Leaf(lf2));
                Trie::Bin(TrieBin{left:r0, right:r1, bits:bits, name:n})
            },
            (Trie::Leaf(_l), Trie::Bin(_r)) => {
                panic!("TODO")
                /*
                assert!(r.bits == bits);
                let lbits = Bits{len:bits.len+1, bits: bits.bits };
                let rbits = Bits{len:bits.len+1, bits:(1 << bits.len) & bits.bits };
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (l0, l1) = Self::split_map(l.map, &bits, e0, e1);
                let lf0 = TrieLeaf{hash:Self::hash_map(&l0), map:l0};
                let lf1 = TrieLeaf{hash:Self::hash_map(&l1), map:l1};
                let (n1, n2) = name_fork(n.clone());

                let l = cell(n1, Self::union_rec(TrieLeaf{, get!(l.right), lbits, l.name));
                let r = cell(n2, Self::union_rec(get!(r.left), get!(r.right), rbits, r.name));
                */
            },
            (Trie::Bin(_l), Trie::Leaf(r)) => {
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (r0, r1) = Self::split_map(r.map, &bits, e0, e1);
                
                drop((r0, r1));
                panic!("")
            },
            (Trie::Bin(l), Trie::Bin(r)) => {
                assert!(l.bits == bits);
                assert!(l.bits == r.bits);
                let lbits = Bits{len:bits.len+1, bits: bits.bits };
                let rbits = Bits{len:bits.len+1, bits:(1 << bits.len) & bits.bits };
                let (n1, n2) = name_fork(n.clone());
                let l = cell(n1, Self::union_rec(get!(l.left), get!(l.right), lbits, l.name));
                let r = cell(n2, Self::union_rec(get!(r.left), get!(r.right), rbits, r.name));
                Trie::Bin(TrieBin{ left:l, right:r, name:n, bits:bits })
            }
        }
    }               
}
