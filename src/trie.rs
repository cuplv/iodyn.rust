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

#[derive(PartialEq,Eq,Clone,Debug,Hash)]
pub struct Trie
    <K:'static+Hash+Eq+Clone+Debug,
     V:'static+Hash+Eq+Clone+Debug>
{
    gauge:usize,
    rec:TrieRec<K,V>,
}

#[derive(PartialEq,Eq,Clone,Debug, Hash)]
enum TrieRec<K:'static+Hash+Eq+Clone+Debug,
             V:'static+Hash+Eq+Clone+Debug>
{
    Empty,
    Leaf(TrieLeaf<K,V>),
    Bin(TrieBin<K,V>),
}
#[derive(PartialEq,Eq,Clone,Debug)]
struct TrieLeaf<K:'static+Hash+Eq+Clone+Debug,
                V:'static+Hash+Eq+Clone+Debug> {
    map:HashMap<K,V>,
}
#[derive(Hash,PartialEq,Eq,Clone,Debug)]
struct TrieBin<K:'static+Hash+Eq+Clone+Debug,
               V:'static+Hash+Eq+Clone+Debug> {
    bits:   Bits,
    name:   Name,
    left:   Art<TrieRec<K,V>>,
    right:  Art<TrieRec<K,V>>,
}

impl<K:'static+Hash+Eq+Clone+Debug,
     V:'static+Hash+Eq+Clone+Debug>
    Hash for TrieLeaf<K,V> 
{
    fn hash<H:Hasher>(&self, _h:&mut H) {
        unimplemented!()
    }
}

impl<K:'static+Hash+Eq+Clone+Debug,
     V:'static+Hash+Eq+Clone+Debug> Trie<K,V> {

    pub fn find (&self, k:&K) -> Option<V> {
        Self::find_hash(self, HashVal(my_hash(k) as usize), k)
    }

    pub fn find_hash (t: &Trie<K,V>, h:HashVal, k:&K) -> Option<V> {
        Self::find_rec(t, &t.rec, h, k)
    }

    fn find_rec (t: &Trie<K,V>, r:&TrieRec<K,V>, h:HashVal, k:&K) -> Option<V> {
        match r {
            &TrieRec::Empty => None,
            &TrieRec::Leaf(ref l) => match l.map.get(k) { Some(v) => Some(v.clone()), None => None },
            &TrieRec::Bin(ref b) => {
                if h.0 & 1 == 0 {
                    Self::find_rec(t, &get!(b.left), HashVal(h.0 >> 1), k)
                } else { 
                    Self::find_rec(t, &get!(b.right), HashVal(h.0 >> 1), k)
                }
            }
        }
    }

    fn split_map (map: HashMap<K,V>, 
                  bits_len:u32,
                  mut map0:HashMap<K,V>, 
                  mut map1:HashMap<K,V>)
                  -> (HashMap<K,V>, HashMap<K,V>) 
    {
        let mask : u64 = make_mask(bits_len as usize) as u64;
        for (k,v) in map.into_iter() {
            let k_hash = my_hash(&k);
            //assert_eq!((mask & k_hash) >> 1, bits.bits as u64); // XXX/???
            if 0 == (k_hash & (1 << bits_len)) {
                map0.insert(k, v);
            } else {
                map1.insert(k, v);
            }
        };
        (map0, map1)
    }

    pub fn empty (gauge:usize) -> Self { 
        Trie{gauge:gauge, rec:TrieRec::Empty}
    }

    pub fn from_hashmap(hm:HashMap<K,V>) -> Self { 
        Trie{gauge:hm.len(), 
             rec:TrieRec::Leaf(TrieLeaf{map:hm})}
    }

    pub fn join (lt: Self, rt: Self, n:Name) -> Self {
        //assert_eq!(lt.gauge, rt.gauge); // ??? -- Or take the min? Or the max? Or the average?
        let lt_rec = lt.rec;
        let lt = Trie{rec:TrieRec::Empty, .. lt};
        Trie{rec:Self::join_rec(&lt, lt_rec, rt.rec, Bits{len:0, bits:0}, n),..lt}
    }

    fn split_bits (bits:&Bits) -> (Bits, Bits) {
        let lbits = Bits{len:bits.len+1, bits: bits.bits };
        let rbits = Bits{len:bits.len+1, bits:(1 << bits.len) & bits.bits };
        (lbits, rbits)
    }

    fn join_rec (t:&Trie<K,V>, lt: TrieRec<K,V>, rt: TrieRec<K,V>, bits:Bits, n:Name) -> TrieRec<K,V> {
        println!("{:?} {:?}", bits, n);
        match (lt, rt) {
            (TrieRec::Empty, rt) => rt,
            (lt, TrieRec::Empty) => lt,
            (TrieRec::Leaf(mut l), TrieRec::Leaf(r)) => {
                if l.map.len() + r.map.len() < t.gauge {
                    // Sub-Case: the leaves, when combined, are smaller than the gauge.
                    for (k,v) in r.map.into_iter() { l.map.insert(k,v); }
                    TrieRec::Leaf(l)
                } else {
                    // Sub-Case: the leaves are large enough to justify not being combined.
                    let (lb, rb) = Self::split_bits(&bits);
                    let (e0, e1) = (HashMap::new(), HashMap::new());
                    let (l0, l1) = Self::split_map(l.map, bits.len + 1, e0, e1);
                    let (r0, r1) = Self::split_map(r.map, bits.len + 1, l0, l1);
                    let (n1, n2) = name_fork(n.clone());
                    let r0 = cell(n1, TrieRec::Leaf(TrieLeaf{map:r0}));
                    let r1 = cell(n2, TrieRec::Leaf(TrieLeaf{map:r1}));
                    TrieRec::Bin(TrieBin{left:r0, right:r1, bits:bits, name:n})
                }
            },
            (TrieRec::Leaf(l), TrieRec::Bin(r)) => {
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (l0, l1) = Self::split_map(l.map, bits.len + 1, e0, e1);
                let (lb, rb) = Self::split_bits(&bits);
                let (n1, n2) = name_fork(n.clone());
                let (m0, m1) = name_fork(r.name.clone());
                let ol = cell(n1, Self::join_rec(t, TrieRec::Leaf(TrieLeaf{map:l0}), get!(r.left),  lb, m0));
                let or = cell(n2, Self::join_rec(t, TrieRec::Leaf(TrieLeaf{map:l1}), get!(r.right), rb, m1));
                TrieRec::Bin(TrieBin{ left:ol, right:or, name:n, bits:bits })
            },
            (TrieRec::Bin(l), TrieRec::Leaf(r)) => {
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (r0, r1) = Self::split_map(r.map, bits.len + 1, e0, e1);
                let (lb, rb) = Self::split_bits(&bits);
                let (n1, n2) = name_fork(n.clone());
                let (m0, m1) = name_fork(l.name.clone());
                let ol = cell(n1, Self::join_rec(t, get!(l.left),  TrieRec::Leaf(TrieLeaf{map:r0}), lb, m0));
                let or = cell(n2, Self::join_rec(t, get!(l.right), TrieRec::Leaf(TrieLeaf{map:r1}), rb, m1));
                TrieRec::Bin(TrieBin{ left:ol, right:or, name:n, bits:bits })
            },
            (TrieRec::Bin(l), TrieRec::Bin(r)) => {
                assert!(l.bits == bits);
                assert!(l.bits == r.bits);
                let (n1, n2) = name_fork(n.clone());
                let (lb, rb) = Self::split_bits(&bits);
                let ol = cell(n1, Self::join_rec(t, get!(l.left),  get!(r.left),  lb, l.name));
                let or = cell(n2, Self::join_rec(t, get!(l.right), get!(r.right), rb, r.name));
                TrieRec::Bin(TrieBin{ left:ol, right:or, name:n, bits:bits })
            }
        }
    }               
}

fn at_leaf(v:&Vec<usize>) -> Trie<usize,()> {
    let mut hm = HashMap::new();
    for x in v { hm.insert(*x,()); }
    Trie::from_hashmap(hm)
}

fn at_bin(l:Trie<usize,()>,lev:u32,n:Option<Name>,r:Trie<usize,()>) -> Trie<usize,()> {
    Trie::join(l,r,n.unwrap())
}

#[test]
pub fn test_join () {
    use rand::{thread_rng,Rng};
    use adapton::engine::*;
    use archive_stack::*;
    use raz::*;
    use memo::*;
    use level_tree::*;
    use raz_meta::Count;
    use std::rc::Rc;

    let mut rng = thread_rng();

    let mut elms : AStack<usize,_> = AStack::new();
    let mut elmv : Vec<usize> = vec![];
    for i in 0..20 {
        let elm = rng.gen::<usize>();
        elmv.push(elm);
        elms.push(elm);
        if i % 3 == 0 {
            elms.archive(Some(name_of_usize(i)), gen_branch_level(&mut rng));
        }
    }
    let tree: RazTree<_,Count> = RazTree::memo_from(&AtHead(elms));
    let trie = tree.fold_up_gauged(Rc::new(at_leaf),Rc::new(at_bin)).unwrap();

    for i in elmv {
        println!("find {:?}", i);
        assert_eq!(trie.find(&i), Some(()));
    }
}
