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

fn my_hash<T>(obj: T) -> HashVal
  where T: Hash
{
  let mut hasher = DefaultHasher::new();
  obj.hash(&mut hasher);
  HashVal(hasher.finish() as usize)
}

/// A hash value -- We define a custom Debug impl for this type.
#[derive(Clone,Hash,Eq,PartialEq)]
pub struct HashVal(usize);

impl fmt::Debug for HashVal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:b}", self.0 & 0b1111)
    }
}

#[derive(PartialEq,Eq,Clone,Hash)]
struct Bits {bits:u32, len:u32}

impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bits{{bits:{:b}, len:{}}}", self.bits, self.len)
    }
}

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
    map:HashMap<K,(HashVal,V)>,
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
        Self::find_hash(self, my_hash(k), k)
    }

    pub fn find_hash (t: &Trie<K,V>, h:HashVal, k:&K) -> Option<V> {
        Self::find_rec(t, &t.rec, h, k)
    }

    fn find_rec (t: &Trie<K,V>, r:&TrieRec<K,V>, h:HashVal, k:&K) -> Option<V> {
        match r {
            &TrieRec::Empty => None,
            &TrieRec::Leaf(ref l) => match l.map.get(k) { 
                None => None,
                Some(&(_,ref v)) => Some(v.clone()),
            },
            &TrieRec::Bin(ref b) => {
                if h.0 & 1 == 0 {
                    Self::find_rec(t, &get!(b.left), HashVal(h.0 >> 1), k)
                } else { 
                    Self::find_rec(t, &get!(b.right), HashVal(h.0 >> 1), k)
                }
            }
        }
    }

    fn split_map (map: HashMap<K,(HashVal,V)>, 
                  bits_len:u32,
                  mut map0:HashMap<K,(HashVal,V)>, 
                  mut map1:HashMap<K,(HashVal,V)>)
                  -> (HashMap<K,(HashVal,V)>, HashMap<K,(HashVal,V)>) 
    {
        //let mask : u64 = make_mask(bits_len as usize) as u64;
        for (k,(k_hash,v)) in map.into_iter() {
            //assert_eq!((mask & k_hash) >> 1, bits.bits as u64); // XXX/???
            if 0 == (k_hash.0 & (1 << bits_len)) {
                map0.insert(k, (k_hash,v));
            } else {
                map1.insert(k, (k_hash,v));
            }
        };
        (map0, map1)
    }

    pub fn empty (gauge:usize) -> Self { 
        Trie{gauge:gauge, rec:TrieRec::Empty}
    }

    pub fn from_hashmap(hm:HashMap<K,(HashVal,V)>) -> Self { 
        Trie{gauge:hm.len(), 
             rec:TrieRec::Leaf(TrieLeaf{map:hm})}
    }

    pub fn join (lt: Self, rt: Self, n:Name) -> Self {
        //assert_eq!(lt.gauge, rt.gauge); // ??? -- Or take the min? Or the max? Or the average?
        let gauge = if lt.gauge > rt.gauge { lt.gauge } else { rt.gauge };
        let lt_rec = lt.rec;
        let lt = Trie{gauge:gauge, rec:TrieRec::Empty};
        Trie{rec:Self::join_rec(&lt, lt_rec, rt.rec, Bits{len:0, bits:0}, n),..lt}
    }

    fn split_bits (bits:&Bits) -> (Bits, Bits) {
        let lbits = Bits{len:bits.len+1, bits:/* zero ------ */ bits.bits };
        let rbits = Bits{len:bits.len+1, bits:(1 << bits.len) | bits.bits };
        (lbits, rbits)
    }

    fn join_rec (t:&Trie<K,V>, lt: TrieRec<K,V>, rt: TrieRec<K,V>, bits:Bits, n:Name) -> TrieRec<K,V> {
        match (lt, rt) {
            (TrieRec::Empty, rt) => rt,
            (lt, TrieRec::Empty) => lt,
            (TrieRec::Leaf(mut l), TrieRec::Leaf(r)) => {
                if l.map.len() == 0 { 
                    TrieRec::Leaf(r)
                } else if r.map.len() == 0 {
                    TrieRec::Leaf(l)
                } else if l.map.len() + r.map.len() < t.gauge {
                    // Sub-Case: the leaves, when combined, are smaller than the gauge.
                    for (k,(k_hash,v)) in r.map.into_iter() { 
                        l.map.insert(k,(k_hash,v));
                    }
                    TrieRec::Leaf(l)
                } else {
                    // Sub-Case: the leaves are large enough to justify not being combined.
                    let (e0, e1) = (HashMap::new(), HashMap::new());
                    let (l0, l1) = Self::split_map(l.map, bits.len, e0, e1);
                    let (r0, r1) = Self::split_map(r.map, bits.len, l0, l1);
                    let (n1, n2) = name_fork(n.clone());
                    let t0 = cell(n1, TrieRec::Leaf(TrieLeaf{map:r0}));
                    let t1 = cell(n2, TrieRec::Leaf(TrieLeaf{map:r1}));
                    TrieRec::Bin(TrieBin{left:t0, right:t1, bits:bits, name:n})
                }
            },
            (TrieRec::Leaf(l), TrieRec::Bin(r)) => {
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (l0, l1) = Self::split_map(l.map, bits.len, e0, e1);
                let (b0, b1) = Self::split_bits(&bits);
                let (n0, n1) = name_fork(n.clone());
                let (m0, m1) = name_fork(r.name.clone());
                let o0 = cell(n0, Self::join_rec(t, TrieRec::Leaf(TrieLeaf{map:l0}), get!(r.left),  b0, m0));
                let o1 = cell(n1, Self::join_rec(t, TrieRec::Leaf(TrieLeaf{map:l1}), get!(r.right), b1, m1));
                TrieRec::Bin(TrieBin{ left:o0, right:o1, name:n, bits:bits })
            },
            (TrieRec::Bin(l), TrieRec::Leaf(r)) => {
                let (e0, e1) = (HashMap::new(), HashMap::new());
                let (r0, r1) = Self::split_map(r.map, bits.len, e0, e1);
                let (b0, b1) = Self::split_bits(&bits);
                let (n0, n1) = name_fork(n.clone());
                let (m0, m1) = name_fork(l.name.clone());
                let o0 = cell(n0, Self::join_rec(t, get!(l.left),  TrieRec::Leaf(TrieLeaf{map:r0}), b0, m0));
                let o1 = cell(n1, Self::join_rec(t, get!(l.right), TrieRec::Leaf(TrieLeaf{map:r1}), b1, m1));
                TrieRec::Bin(TrieBin{ left:o0, right:o1, name:n, bits:bits })
            },
            (TrieRec::Bin(l), TrieRec::Bin(r)) => {
                assert!(l.bits == bits);
                assert!(l.bits == r.bits);
                let (n1, n2) = name_fork(n.clone());
                let (b0, b1) = Self::split_bits(&bits);
                let o0 = cell(n1, Self::join_rec(t, get!(l.left),  get!(r.left),  b0, l.name));
                let o1 = cell(n2, Self::join_rec(t, get!(l.right), get!(r.right), b1, r.name));
                TrieRec::Bin(TrieBin{ left:o0, right:o1, name:n, bits:bits })
            }
        }
    }               
}


#[test]
pub fn test_join () {
    fn at_leaf(v:&Vec<usize>) -> Trie<usize,()> {
        let mut hm = HashMap::new();
        for x in v { 
            let x_hash = my_hash(&x);
            hm.insert(*x,(x_hash,()));
        }
        Trie::from_hashmap(hm)
    }    
    fn at_bin(l:Trie<usize,()>,_lev:u32,n:Option<Name>,r:Trie<usize,()>) -> Trie<usize,()> {
        Trie::join(l,r,n.unwrap())
    }
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
    for i in 0..1000 {
        let elm = rng.gen::<usize>() % 100;
        elmv.push(elm);
        elms.push(elm);
        if i % 1 == 0 {
            elms.archive(Some(name_of_usize(i)), gen_branch_level(&mut rng));
        }
    }
    let tree: RazTree<_,Count> = RazTree::memo_from(&AtHead(elms));
    //println!("{:?}\n", tree);

    let trie = tree.fold_up_gauged(Rc::new(at_leaf),Rc::new(at_bin)).unwrap();
    //println!("{:?}\n", trie);

    for i in elmv {
        //println!("find {:?}", i);
        assert_eq!(trie.find(&i), Some(()));
    }
}
