//! Incremental, high-gauge finite maps, for use by the Archivist in Adapton.
//!
//! Conceptually, these finite maps are tries.
//! Concretely, they consist of skip-list-like structures.

use std::mem;
use std::fmt;
use std::rc::Rc;
use std::fmt::Debug;
use std::hash::{Hash,Hasher};
use adapton::engine::{cell,force,Art,Name,name_pair,name_of_usize};
use adapton::macros::{my_hash};

/// A hash value -- We define a custom Debug impl for this type.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct HashVal(usize);

#[derive(Debug,Clone,Hash,Eq,PartialEq)]
enum PathIdx<K,V> {
    /// Empty sub-skiplist; every key maps to None
    Empty,
    /// Non-empty sub-skiplist
    Path(Art<Path<K,V>>),
}

/// A contiguous block of skiplist paths/keys/values
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Path<K,V> {
    hash:   HashVal,
    kvs:    Vec<(K,Option<V>)>,
    paths:  Vec<Option<Art<Path<K,V>>>>,
}

/// The gauged, incremental Skiplist.  This structure maintains the head
/// of the skip list, a name and a counter.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct Skiplist<K,V> {
    path_len: usize,
    name: Name,
    cntr: usize,
    head: Art<Option<Path<K,V>>>,
}

/// Cursor state for traversing the skiplist
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Cursor<K,V> {
    /// Bit/path-position index; Invariant: increases monotonically
    bit_idx: usize,
    /// Invariant: `len(paths) ==  bit_idx`
    paths: Vec<Option<Art<Path<K,V>>>>,
}


impl<K:Clone,V:Clone> Cursor<K,V> {
    fn new() -> Self {
        Cursor{
            bit_idx: 0,
            paths: vec![],            
        }
    }
    fn fill_empty(&mut self, bits:usize) {
        for _ in self.bit_idx..bits {
            self.paths.push(None)
        }
    }
}


// impl<K:'static+Hash+Eq+Debug+Clone,V:'static+Hash+Eq+Debug+Clone> Chunks<K,V> {
//     fn get_cursor(&self, cur:&mut Cursor<K,V>, key:K, key_hash:HashVal) -> Option<Option<V>> {
//         match *self {
//             Chunks::Chunk(ref chk) => chk.get_cursor(None, cur, key, key_hash),
//             Chunks::Link(ref lnk) => {
//                 let cur_art = &lnk.link;
//                 (force(&cur_art)).get_cursor(Some(cur_art.clone()), cur, key, key_hash)
//             },
//         }
//     }
// }

impl<K:'static+Hash+Eq+Debug+Clone,
     V:'static+Hash+Eq+Debug+Clone> Path<K,V> {
   
    fn build_path
        (self,                    
         path_len:usize,
         cur_art:Option<Art<Path<K,V>>>, 
         cur:&mut Cursor<K,V>, 
         key:&K, key_hash:HashVal) -> Option<Vec<(K,Option<V>)>>
    {
        // Mutable copies of these bit strings
        let mut key_bits = key_hash.0;
        let mut hsh_bits = self.hash.0.clone();
        
        // Discard bits that we've already "traversed"
        key_bits >>= cur.bit_idx;
        hsh_bits >>= cur.bit_idx;

        let same_hash = { hsh_bits == key_bits };
        // Check for perfect match of remaining bits
        if same_hash {
            // Copy the remaining paths for this chk_idx into the cursor
            for i in cur.bit_idx..path_len { cur.paths.push(self.paths.get(i).unwrap().clone()) };
            //for (k,vo) in self.kvs { if key == k { return Some(vo) } else { continue } };
            //unreachable!("found matching hash; compared all keys; should be done.")
            return Some(self.kvs.clone());
        } else {    
            // While bits match, move the cursor along axis cur.bit_idx;
            // When bits mis-match, move cursor along axis cur.chk_idx, and recur via `get_cursor`.
            let start_idx = cur.bit_idx;
            'matching_bits: 
            for i in start_idx..path_len {
                let pi : &Option<Art<Path<K,V>>> = self.paths.get(i).unwrap();
                if (key_bits & 0x1) == (hsh_bits & 0x1) {
                    cur.paths.push(pi.clone());
                    key_bits >>= 1;
                    continue 'matching_bits;
                } else {
                    cur.paths.push(cur_art);
                    match * self.paths.get(i).unwrap() {
                        None => {
                            cur.bit_idx = i+1;
                            cur.fill_empty(path_len);
                            return None
                        }
                        Some(ref a) => {
                            let c = force(a);
                            cur.bit_idx = i+1;
                            return c.build_path(path_len, Some(a.clone()), cur, key, key_hash)
                        }
                    }
                }
            };
            unreachable!("compared all bits; should be done")
        }
    }       
}


/// Abstract, finite map interface implemented by the incremental, high-gauge Skiplist.
pub trait FinMap<K,V>
{
    /// An empty mapping; all keys map to `None`.
    fn emp(path_len:usize, nm:Name) -> Self;
    /// Name and articulate the Skiplist
    fn archive(&mut self, nm:Name);
    /// Extend mapping to map key `k` to optional value `opv`. 
    /// Returns the prior mapping of key `k`, if any.
    fn ext(&mut self, k:K, opv:Option<V>) -> Option<V>;
    /// Extend mapping to map key `k` to value `v`. This is a special case of `ext`.
    fn put(&mut self, k:K, v:V);
    /// Remove `k` from the mapping; Afterward, it maps to
    /// `None`. Returns the prior mapping of key `k`, if any. This is
    /// a special case of `ext`.
    fn rem(&mut self, k:K) -> Option<V>;
    /// Map key `k` to its corresponding value in the mapping, if any.
    fn get(&self, k:K) -> Option<V>;
}

impl<K:'static+Eq+Clone+Debug+Hash,
     V:'static+Eq+Clone+Debug+Hash> 
    FinMap<K,V> 
    for Skiplist<K,V> 
{    
    fn emp(path_len:usize, n:Name) -> Self { 
        Skiplist{
            path_len:path_len,
            name:n.clone(),
            cntr:1,
            head:cell(name_pair(n, name_of_usize(0)), None),
        }
    }
   
    fn archive(&mut self, n:Name) {
        self.name = n;
        self.cntr = 0;
    }

    fn ext(&mut self, k:K, opv:Option<V>) -> Option<V> {
        let k_hash = HashVal(my_hash(&k) as usize);
        let mut cur = Cursor::new();
        let old_kvs = match force(&self.head) {
            None       => None,
            Some(path) => path.build_path(self.path_len, None, &mut cur, &k, k_hash.clone()),
        };
        let mut new_kvs = vec![];
        let mut opv_old = None;
        match old_kvs {
            Some(kvs) => {
                for (k0,opv0) in kvs.into_iter() {
                    if &k == &k0 {
                        new_kvs.push((k0, opv.clone()));
                        opv_old = Some(opv0);
                    } else {
                        new_kvs.push((k0, opv0));
                    }
                }},
            None => {
                new_kvs.push((k, opv))
            }
        };
        let new_path = {
            panic!("TODO")
        };
        self.head = 
            cell(name_pair(self.name.clone(), 
                           name_of_usize(self.cntr)),
                 Some(new_path));
        self.cntr += 1;
        return opv_old.unwrap_or(None);
    }

    fn rem(&mut self, k:K) -> Option<V> {
        self.ext(k, None)
    }

    fn put(&mut self, k:K, v:V) {
        let _ = self.ext(k, Some(v));
    }

    fn get(&self, k:K) -> Option<V> {
        let k_hash = HashVal(my_hash(&k) as usize);
        let mut cur = Cursor::new();
        let res = match force(&self.head) {
            None       => None,
            Some(path) => path.build_path(self.path_len, None, &mut cur, &k, k_hash),
        };            
        match res {
            Some(kvs) => {
                for (k0,opval) in kvs.into_iter() {
                    if &k == &k {
                        return opval
                    }
                };
                return None
            },
            None => None,
        }
    }
}

// #[test]
// fn skiplist_opt_test () {
//     use std::collections::HashMap;
//     use rand::{Rng,thread_rng};
//     use adapton::engine::{manage,name_of_usize};
//     let mut rng = thread_rng();
//     let numops = 10000;
//     let numkeys = 100;
//     let gauged = true;
//     let gauge = 100;
    
//     manage::init_dcg();
    
//     let mut m = HashMap::new();
//     let mut t = Skiplist::emp();
    
//     for i in 0..numops {        
//         let r1 : usize = rng.gen(); let r1 = r1 % numkeys;
//         let r2 : usize = rng.gen(); let r2 = r2 % numkeys;
//         let nm = if gauged && i % gauge == 0 { Some(name_of_usize(i)) } else { None };

//         // Test random insertion
//         if !(nm == None) { println!("=========\nname {:?}:", nm); };
//         println!("insert #{:?}: key {:?} maps to {:?}", i, r1, r2);
//         m.insert(r1, r2);
//         t.put(r1, r2);
//         match nm {
//             Some(nm) => t.archive(nm),
//             None => (),
//         };

//         // Test random lookup        
//         let r3 : usize = rng.gen(); 
//         let r3 = r3 % (numkeys * 2); // Look for non-existent keys with prob 0.5
//         println!("lookup #{:?}: key {:?} maps to {:?}", i, r3, m.get(&r3));
//         assert_eq!(m.get(&r3).map(|&n|n.clone()), t.get(r3));
//     }
// }


// #[test]
// fn skiplist_opt_tiny () {
//     use adapton::engine::name_of_usize;
//     let mut c = Skiplist::emp();
//     c.put(1, 1);
//     println!("{:?}\n", c);    
//     c.put(2, 2);
//     println!("{:?}\n", c);    
//     c.put(3, 3);
//     println!("{:?}\n", c);    
//     c.put(4, 4);
//     c.archive(name_of_usize(4));
//     println!("{:?}\n", c);
//     c.put(5, 5);
//     println!("{:?}\n", c);
//     c.put(6, 6);
//     println!("{:?}\n", c);

//     assert_eq!(c.get(0), None);
//     assert_eq!(c.get(1), Some(1));
//     assert_eq!(c.get(2), Some(2));
//     assert_eq!(c.get(3), Some(3));
//     assert_eq!(c.get(4), Some(4));
//     assert_eq!(c.get(5), Some(5));
//     assert_eq!(c.get(6), Some(6));
// }

// #[test]
// fn skiplist_opt_small () {
//     let mut c = Skiplist::emp();
//     c.put(1, 1);
//     c.put(2, 2);
//     c.put(3, 3);
//     c.put(4, 4);
//     c.put(5, 5);
//     c.put(6, 6);

//     assert_eq!(c.get(0), None);
//     assert_eq!(c.get(1), Some(1));
//     assert_eq!(c.get(2), Some(2));
//     assert_eq!(c.get(3), Some(3));
//     assert_eq!(c.get(4), Some(4));
//     assert_eq!(c.get(5), Some(5));
//     assert_eq!(c.get(6), Some(6));

//     c.put(11, 11);
//     c.put(12, 12);
//     c.put(13, 13);
//     c.put(14, 14);
//     c.put(15, 15);
//     c.put(16, 16);

//     assert_eq!(c.get(0), None);
//     assert_eq!(c.get(1), Some(1));
//     assert_eq!(c.get(2), Some(2));
//     assert_eq!(c.get(3), Some(3));
//     assert_eq!(c.get(4), Some(4));
//     assert_eq!(c.get(5), Some(5));
//     assert_eq!(c.get(6), Some(6));
    
//     assert_eq!(c.get(7),  None);
//     assert_eq!(c.get(8),  None);
//     assert_eq!(c.get(9),  None);
//     assert_eq!(c.get(10), None);

//     assert_eq!(c.get(11), Some(11));
//     assert_eq!(c.get(12), Some(12));
//     assert_eq!(c.get(13), Some(13));
//     assert_eq!(c.get(14), Some(14));
//     assert_eq!(c.get(15), Some(15));
//     assert_eq!(c.get(16), Some(16));

//     assert_eq!(c.get(17), None);
//     assert_eq!(c.get(18), None);
//     assert_eq!(c.get(19), None);
//     assert_eq!(c.get(20), None);

//     c.put(21, 21);
//     c.put(22, 22);
//     c.put(23, 23);
//     c.put(24, 24);
//     c.put(25, 25);
//     c.put(26, 26);

//     assert_eq!(c.get(0), None);
//     assert_eq!(c.get(1), Some(1));
//     assert_eq!(c.get(2), Some(2));
//     assert_eq!(c.get(3), Some(3));
//     assert_eq!(c.get(4), Some(4));
//     assert_eq!(c.get(5), Some(5));
//     assert_eq!(c.get(6), Some(6));
    
//     assert_eq!(c.get(7),  None);
//     assert_eq!(c.get(8),  None);
//     assert_eq!(c.get(9),  None);
//     assert_eq!(c.get(10), None);

//     assert_eq!(c.get(11), Some(11));
//     assert_eq!(c.get(12), Some(12));
//     assert_eq!(c.get(13), Some(13));
//     assert_eq!(c.get(14), Some(14));
//     assert_eq!(c.get(15), Some(15));
//     assert_eq!(c.get(16), Some(16));

//     assert_eq!(c.get(17), None);
//     assert_eq!(c.get(18), None);
//     assert_eq!(c.get(19), None);
//     assert_eq!(c.get(20), None);

//     assert_eq!(c.get(21), Some(21));
//     assert_eq!(c.get(22), Some(22));
//     assert_eq!(c.get(23), Some(23));
//     assert_eq!(c.get(24), Some(24));
//     assert_eq!(c.get(25), Some(25));
//     assert_eq!(c.get(26), Some(26));

//     assert_eq!(c.get(27), None);
//     assert_eq!(c.get(28), None);
//     assert_eq!(c.get(29), None);
//     assert_eq!(c.get(30), None);
// }
