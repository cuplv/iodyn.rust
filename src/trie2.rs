//! Binary Hash Tries, for representing sets and finite maps.
//!
//! Suitable for the Archivist role in Adapton.
//!
// Matthew Hammer <Matthew.Hammer@Colorado.edu>

//use std::rc::Rc;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash,Hasher};
//use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use adapton::engine::*;
use adapton::macros::*;

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
    <K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
{
    meta:TrieMeta,
    rec:TrieRec<K,V>,
}

#[derive(PartialEq,Eq,Clone,Debug,Hash)]
pub struct TrieMeta {
    gauge:usize,
}

#[derive(PartialEq,Eq,Clone,Debug, Hash)]
enum TrieRec<K:'static+Hash+PartialEq+Eq+Clone+Debug,
             V:'static+Hash+PartialEq+Eq+Clone+Debug>
{
    Empty,
    Leaf(TrieLeaf<K,V>),
    Bin(TrieBin<K,V>),
}
#[derive(PartialEq,Eq,Clone,Debug)]
struct TrieLeaf<K:'static+Hash+PartialEq+Eq+Clone+Debug,
                V:'static+Hash+PartialEq+Eq+Clone+Debug> {
    kvs:Rc<Vec<(K,HashVal,V)>>,
}
#[derive(Hash,PartialEq,Eq,Clone,Debug)]
struct TrieBin<K:'static+Hash+PartialEq+Eq+Clone+Debug,
               V:'static+Hash+PartialEq+Eq+Clone+Debug> {
    bits:   Bits,
    name:   Name,
    left:   Art<TrieRec<K,V>>,
    right:  Art<TrieRec<K,V>>,
}

impl<K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug>
    Hash for TrieLeaf<K,V> 
{
    fn hash<H:Hasher>(&self, _h:&mut H) {
        unimplemented!()
    }
}

impl<K:'static+Hash+PartialEq+Eq+Clone+Debug,
     V:'static+Hash+PartialEq+Eq+Clone+Debug> Trie<K,V> {

    pub fn find (&self, k:&K) -> Option<V> {
        Self::find_hash(self, my_hash(k), k)
    }

    pub fn find_hash (t: &Trie<K,V>, h:HashVal, k:&K) -> Option<V> {
        Self::find_rec(t, &t.rec, h.clone(), h, k)
    }

    fn find_rec (t: &Trie<K,V>, r:&TrieRec<K,V>, h:HashVal, h_rest:HashVal, k:&K) -> Option<V> {
        match r {
            &TrieRec::Empty => None,
            &TrieRec::Leaf(ref l) => {
                let mut ans = None;
                for &(ref k2,ref k2_hash,ref v) in l.kvs.iter() {
                    if k2_hash == &h && k2 == k {
                        ans = Some(v.clone())
                    }
                }
                return ans
            },
            &TrieRec::Bin(ref b) => {
                if h_rest.0 & 1 == 0 {
                    Self::find_rec(t, &get!(b.left), h, HashVal(h_rest.0 >> 1), k)
                } else { 
                    Self::find_rec(t, &get!(b.right), h, HashVal(h_rest.0 >> 1), k)
                }
            }
        }
    }

    fn split_vec (vec: Rc<Vec<(K,HashVal,V)>>,
                  bits_len:u32,
                  mut vec0:Vec<(K,HashVal,V)>, 
                  mut vec1:Vec<(K,HashVal,V)>)
                  -> (Vec<(K,HashVal,V)>, Vec<(K,HashVal,V)>)
    {
        //let mask : u64 = make_mask(bits_len as usize) as u64;
        for &(ref k, ref k_hash, ref v) in vec.iter() {
            //assert_eq!((mask & k_hash) >> 1, bits.bits as u64); // XXX/???
            if 0 == (k_hash.0 & (1 << bits_len)) {
                vec0.push((k.clone(),k_hash.clone(),v.clone()))
            } else {
                vec1.push((k.clone(),k_hash.clone(),v.clone()))
            }
        };
        (vec0, vec1)
    }

    fn meta (gauge:usize) -> TrieMeta {
        TrieMeta{gauge:gauge}
    }

    pub fn empty (gauge:usize) -> Self { 
        Trie{meta:Self::meta(gauge), rec:TrieRec::Empty}
    }

    pub fn from_vec(vec_in:&Vec<(K,V)>) -> Self { 
        let mut vec = Vec::new();
        for &(ref k, ref v) in vec_in.iter() {
            let k_hash = my_hash(k);
            vec.push((k.clone(),k_hash,v.clone()));
        };
        Trie{meta:Self::meta(vec.len()), 
             rec:TrieRec::Leaf(TrieLeaf{kvs:Rc::new(vec)})}
    }

    pub fn from_key_vec(vec_in:&Vec<K>) -> Trie<K,()> { 
        let mut vec = Vec::new();
        for k in vec_in.iter() {
            let k_hash = my_hash(k);
            vec.push((k.clone(),k_hash,()));
        };
        Trie{meta:Self::meta(vec.len()), 
             rec:TrieRec::Leaf(TrieLeaf{kvs:Rc::new(vec)})}
    }

    pub fn join (lt: Self, rt: Self, n:Name) -> Self {
        //assert_eq!(lt.gauge, rt.gauge); // ??? -- Or take the min? Or the max? Or the average?
        let gauge = if lt.meta.gauge > rt.meta.gauge { lt.meta.gauge } else { rt.meta.gauge };
        Trie{rec:Self::join_rec(TrieMeta{gauge:gauge}, lt.rec, rt.rec, Bits{len:0, bits:0}, n),..lt}
    }

    fn split_bits (bits:&Bits) -> (Bits, Bits) {
        let lbits = Bits{len:bits.len+1, bits:/* zero ------ */ bits.bits };
        let rbits = Bits{len:bits.len+1, bits:(1 << bits.len) | bits.bits };
        (lbits, rbits)
    }

    // TODO-Soon: Opt: After splitting a vec, create leaves by first checking whether the vec is empty.

    fn leaf_or_empty (kvs:Vec<(K,HashVal,V)>) -> TrieRec<K,V> {
        if kvs.len() == 0 { TrieRec::Empty }
        else { TrieRec::Leaf(TrieLeaf{kvs:Rc::new(kvs)}) }
    }

    fn join_rec (meta:TrieMeta, lt: TrieRec<K,V>, rt: TrieRec<K,V>, bits:Bits, n:Name) -> TrieRec<K,V> {
        match (lt, rt) {
            (TrieRec::Empty, rt) => rt,
            (lt, TrieRec::Empty) => lt,
            (TrieRec::Leaf(l), TrieRec::Leaf(r)) => {
                if l.kvs.len() == 0 { 
                    TrieRec::Leaf(r)
                } else if r.kvs.len() == 0 {
                    TrieRec::Leaf(l)
                } else if l.kvs.len() + r.kvs.len() < meta.gauge {
                    // Sub-Case: the leaves, when combined, are smaller than the gauge.
                    let mut vec = (*l.kvs).clone();
                    for &(ref k, ref k_hash, ref v) in r.kvs.iter() { 
                        vec.push((k.clone(),k_hash.clone(),v.clone()));
                    }
                    Self::leaf_or_empty(vec)
                } else {
                    // Sub-Case: the leaves are large enough to justify not being combined.
                    let (e0, e1) = (Vec::new(), Vec::new());
                    let (l0, l1) = Self::split_vec(l.kvs, bits.len, e0, e1);
                    let (r0, r1) = Self::split_vec(r.kvs, bits.len, l0, l1);
                    let (n1, n2) = name_fork(n.clone());
                    let t0 = cell(n1, TrieRec::Leaf(TrieLeaf{kvs:Rc::new(r0)}));
                    let t1 = cell(n2, TrieRec::Leaf(TrieLeaf{kvs:Rc::new(r1)}));
                    TrieRec::Bin(TrieBin{left:t0, right:t1, bits:bits, name:n})
                }
            },
            (TrieRec::Leaf(l), TrieRec::Bin(r)) => {
                let (e0, e1) = (Vec::new(), Vec::new());
                let (l0, l1) = Self::split_vec(l.kvs, bits.len, e0, e1);
                let (b0, b1) = Self::split_bits(&bits);
                let (n0, n1) = name_fork(n.clone());
                let (m0, m1) = name_fork(r.name.clone());
                let o0 = eager!(n0 =>> Self::join_rec, m:meta.clone(), l:Self::leaf_or_empty(l0), r:get!(r.left),  b:b0, n:m0);
                let o1 = eager!(n1 =>> Self::join_rec, m:meta.clone(), l:Self::leaf_or_empty(l1), r:get!(r.right), b:b1, n:m1);
                TrieRec::Bin(TrieBin{ left:o0.0, right:o1.0, name:n, bits:bits })
            },
            (TrieRec::Bin(l), TrieRec::Leaf(r)) => {
                let (e0, e1) = (Vec::new(), Vec::new());
                let (r0, r1) = Self::split_vec(r.kvs, bits.len, e0, e1);
                let (b0, b1) = Self::split_bits(&bits);
                let (n0, n1) = name_fork(n.clone());
                let (m0, m1) = name_fork(l.name.clone());
                let o0 = eager!(n0 =>> Self::join_rec, m:meta.clone(), l:get!(l.left),  r:Self::leaf_or_empty(r0), b:b0, n:m0);
                let o1 = eager!(n1 =>> Self::join_rec, m:meta.clone(), l:get!(l.right), r:Self::leaf_or_empty(r1), b:b1, n:m1);
                TrieRec::Bin(TrieBin{ left:o0.0, right:o1.0, name:n, bits:bits })
            },
            (TrieRec::Bin(l), TrieRec::Bin(r)) => {
                assert!(l.bits == bits);
                assert!(l.bits == r.bits);
                let (n1, n2) = name_fork(n.clone());
                let (b0, b1) = Self::split_bits(&bits);
                let o0 = eager!(n1 =>> Self::join_rec, m:meta.clone(), l:get!(l.left),  r:get!(r.left), b:b0, n:l.name);
                let o1 = eager!(n2 =>> Self::join_rec, m:meta.clone(), l:get!(l.right), r:get!(r.right), b:b1, n:r.name);
                TrieRec::Bin(TrieBin{ left:o0.0, right:o1.0, name:n, bits:bits })
            }
        }
    }               
}

#[test]
pub fn test_join () {
    fn at_leaf(v:&Vec<usize>) -> Trie<usize,()> {
        Trie::<usize,()>::from_key_vec(v)
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
    for i in 0..8 {
        let elm = rng.gen::<usize>() % 100;
        elmv.push(elm);
        elms.push(elm);
        if i % 1 == 0 {
            elms.archive(Some(name_of_usize(i)), gen_branch_level(&mut rng));
        }
    }
    let tree: RazTree<_,Count> = RazTree::memo_from(&AtHead(elms));
    println!("{:?}\n", tree);

    let trie = tree.fold_up_gauged(Rc::new(at_leaf),Rc::new(at_bin)).unwrap();
    println!("{:?}\n", trie);

    for i in elmv {
        println!("find {:?}", i);
        assert_eq!(trie.find(&i), Some(()));
    }
}
