//! Incremental, high-gauge finite maps, for use by the Archivist in Adapton.
//!
//! Conceptually, these finite maps are selectively-persistent tries.
//! Concretely, they consist of skip-list-like structures.

use std::mem;
use std::fmt;
use std::rc::Rc;
use std::fmt::Debug;
use std::hash::{Hash,Hasher};
use adapton::engine::{cell,force,Art,Name};
use adapton::macros::{my_hash};

/// A numeral index. The cheapest form of address.
/// TODO-Someday: Make this smaller than a machine word?
type Idx = usize;

/// A hash value -- We define a custom Debug impl for this type.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct HashVal(usize);

/// The number of hash bits to use when constructing trie paths.
/// ignore bits that fall beyond this threshold.
///
/// NOTE/TODO/XXX: When we saturate this "full trie", we may fail to
/// distinguish keys that are actually distinct, due to their hash
/// prefixes "colliding".  In these cases, the code below will detect
/// a hash collision and panic.
const MAX_PATH_LEN : usize = 32;

#[derive(Debug)]
struct CloneCounter (usize);
impl Clone for CloneCounter {
    fn clone(&self) -> Self {
        //println!("clone count {:?}", self);
        if false && self.0 > 2 {
            panic!("bad programmer!")
        };
        CloneCounter(self.0 + 1)
    }
}

#[derive(Debug,Clone,Hash,Eq,PartialEq)]
enum PathIdx<K,V> {
    /// Empty sub-trie; every key maps to None
    Empty,
    /// Intra-Chunk sub-trie; the index addresses a vector that is "here", in this chunk.
    Local(Idx),
    /// Inter-Chunk sub-trie; the index addresses a vector that is "there", in another chunk.
    Global(Art<Rc<Chunk<K,V>>>, Idx),
}

/// A contiguous block of trie paths/keys/values
#[derive(Debug,Clone)]
struct Chunk<K,V> {
    cntr:   CloneCounter,
    hash:   u64,
    head:   Option<Idx>,
    keys:   Vec<K>,
    vals:   Vec<Option<V>>,    
    hashes: Vec<HashVal>,
    paths:  Vec<Vec<PathIdx<K,V>>>,
}
/// Articulated head of the skip list
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Link<K,V> {
    name: Option<Name>,
    link: Art<Rc<Chunk<K,V>>>,
}
/// The head of the skip list is either a Chunk, or an optionally-named, articulated Link.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
enum Chunks<K,V> {
    /// Head of skip list is immutable to Archivist
    Link(Link<K,V>),
    /// Head of skip list is mutable to Archivist
    Chunk(Chunk<K,V>),
}
/// The gauged, incremental Trie.  This structure maintains the head
/// of the skip list that represents the trie.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct Trie<K,V> {
    head: Chunks<K,V>
}

/// Cursor state for traversing Chunks of the trie
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Cursor<K,V> {
    /// Intra-chunk index
    chk_idx: Idx,
    /// Bit/path-position index; Invariant: increases monotonically
    bit_idx: Idx,
    /// Invariant: `len(paths) ==  bit_idx`
    paths: Vec<PathIdx<K,V>>,
}

/// High-performance code should avoid this. To make that easier to
/// detect (dynamically), we panic here:
//impl<K,V> Clone for Chunk<K,V> { 
//impl<K:'static+Hash+Eq+Debug+Clone,V:'static+Hash+Eq+Debug+Clone> Chunk<K,V> {
//}

impl<K:Hash,V:Hash> Hash for Chunk<K,V> {
  fn hash<H>(&self, state: &mut H) where H: Hasher {
    self.hash.hash(state)
  }
}
impl<K,V> Eq for Chunk<K,V> { }
impl<K,V> PartialEq for Chunk<K,V> { 
    fn eq(&self, other:&Self) -> bool { 
        // XXX -- This isn't generally sound (due to hash collisions); but it's fast
        self.hash == other.hash
    }
}

impl<K:Clone,V:Clone> Cursor<K,V> {
    fn new() -> Self {
        Cursor{
            chk_idx: 0,
            bit_idx: 0,
            paths: vec![],            
        }
    }        
    fn fill_empty(&mut self) {
        for _ in self.bit_idx..MAX_PATH_LEN {
            self.paths.push(PathIdx::Empty)
        }
    }
}

fn translate_path_idx<K:Clone,V:Clone>
    (cur_art: &Option<Art<Rc<Chunk<K,V>>>>, 
     path_idx: &PathIdx<K,V>) -> PathIdx<K,V>
{
    match *cur_art { 
        None => path_idx.clone(),
        Some(ref a) => {
            match path_idx {
                & PathIdx::Local(j) => PathIdx::Global(a.clone(),j),
                other => other.clone(),
            }
        }
    }
}


impl<K:'static+Hash+Eq+Debug+Clone,V:'static+Hash+Eq+Debug+Clone> Chunks<K,V> {
    fn get_cursor(&self, cur:&mut Cursor<K,V>, key:K, key_hash:HashVal) -> Option<Option<V>> {
        match *self {
            Chunks::Chunk(ref chk) => chk.get_cursor(None, cur, key, key_hash),
            Chunks::Link(ref lnk) => {
                let cur_art = &lnk.link;
                (force(&cur_art)).get_cursor(Some(cur_art.clone()), cur, key, key_hash)
            },
        }
    }
}

impl<K:'static+Hash+Eq+Debug+Clone,V:'static+Hash+Eq+Debug+Clone> Chunk<K,V> {

    fn new() -> Self {
        Chunk{
            cntr:   CloneCounter(0),
            hash:   0,
            head:   None,
            keys:   Vec::new(),
            vals:   Vec::new(),
            hashes: Vec::new(),
            paths:  Vec::new(),
        }
    }
    
    fn get_cursor(&self, 
                  cur_art:Option<Art<Rc<Chunk<K,V>>>>, 
                  cur:&mut Cursor<K,V>, 
                  key:K, key_hash:HashVal) -> Option<Option<V>> 
    {
        match self.head {
            None => {
                cur.fill_empty();
                None
            },
            Some(chk_idx) => {
                cur.chk_idx = chk_idx;
                self.get_rec(cur_art, cur, key, key_hash)
            }
        }
    }

    fn get_rec(&self, 
               cur_art:Option<Art<Rc<Chunk<K,V>>>>, 
               cur:&mut Cursor<K,V>, 
               key:K, key_hash:HashVal) -> Option<Option<V>> 
    {
        assert!(self.keys.len() > 0);

        // Mutable copies of these bit strings
        let mut key_bits = key_hash.0;
        let mut chk_bits = self.hashes.get(cur.chk_idx).unwrap().0.clone();
        
        // Discard bits that we've already "traversed"
        key_bits >>= cur.bit_idx;
        chk_bits >>= cur.bit_idx;

        let same_hash = { chk_bits == key_bits };
        let same_keys = { Some(&key) == self.keys.get(cur.chk_idx) };
        if same_hash && !same_keys {
            panic!("hash collision:\n keys {:?}\n and {:?}\n both hash to {:b}", 
                   key, self.keys.get(cur.chk_idx), key_bits);
        };        
        // Check for perfect match of remaining bits
        if same_hash && same_keys {
            // Copy the remaining paths for this chk_idx into the cursor
            for i in cur.bit_idx..MAX_PATH_LEN {
                cur.paths.push(translate_path_idx(&cur_art, self.paths.get(cur.chk_idx).unwrap().get(i).unwrap()))
            };            
            // Copy the value option for this chk_idx; it's our result
            let valop = self.vals.get(cur.chk_idx).unwrap().clone();
            return Some(valop)
        } else {    
            // While bits match, move the cursor along axis cur.bit_idx;
            // When bits mis-match, move cursor along axis cur.chk_idx, and recur via `get_cursor`.
            let start_idx = cur.bit_idx;
            'matching_bits: 
            for i in start_idx..MAX_PATH_LEN {
                let pi : &PathIdx<K,V> = self.paths.get(cur.chk_idx).unwrap().get(i).unwrap();
                if (key_bits & 0x1) == (chk_bits & 0x1) {
                    cur.paths.push(translate_path_idx(&cur_art, &pi));
                    key_bits >>= 1;
                    chk_bits >>= 1;
                    continue 'matching_bits;
                } else {
                    let pi = PathIdx::Local(cur.chk_idx);
                    cur.paths.push(translate_path_idx(&cur_art, &pi));
                    match * self.paths.get(cur.chk_idx).unwrap().get(i).unwrap() {
                        //                        PathIdx::Active  => unreachable!("Should have taken 'then' branch, in 'if' above"),
                        PathIdx::Empty   => {
                            cur.bit_idx = i+1;
                            cur.fill_empty();
                            return None
                        }
                        PathIdx::Local(j) => {
                            cur.chk_idx = j;
                            cur.bit_idx = i+1;
                            return self.get_rec(cur_art, cur, key, key_hash)
                        },
                        PathIdx::Global(ref a, j) => {
                            let c = force(a);
                            cur.chk_idx = j;
                            cur.bit_idx = i+1;
                            return c.get_rec(Some(a.clone()), cur, key, key_hash)
                        }
                    }
                }
            };
            panic!("ran out of bits to distinguish keys!\n target:{:b} (key: {:?})\n  found:{:b} (key: {:?}),\nHint: To fix this, try increasing the constant MAX_PATH_LEN (currently, {:?}), if you can.", 
                   key_hash.0, key,
                   self.hashes.get(cur.chk_idx).unwrap().0.clone(), self.keys.get(cur.chk_idx).unwrap(),
                   MAX_PATH_LEN);
        }
    }       
}


/// Abstract, finite map interface implemented by the incremental, high-gauge Trie.
pub trait FinMap<K,V>
{
    /// An empty mapping; all keys map to `None`.
    fn emp() -> Self;
    /// Name and articulate the Trie
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
    for Trie<K,V> 
{    
    fn emp() -> Self { Trie{head:Chunks::Chunk(Chunk::new())} }

    fn archive(&mut self, n:Name) {
        let chunks = mem::replace(&mut self.head, Chunks::Chunk(Chunk::new()));
        let hd_chunk : Rc<Chunk<_,_>> = 
            match chunks {
                Chunks::Link(lnk) => force(&lnk.link),
                Chunks::Chunk(c) => Rc::new(c),
            };
        let c = cell(n.clone(), hd_chunk);
        self.head = Chunks::Link(Link{name:Some(n), link:c});
    }

    fn ext(&mut self, k:K, opv:Option<V>) -> Option<V> {
        let k_hash = HashVal(my_hash(&k) as usize);
        let mut cur = Cursor::new();
        let temp_perf_test = false; // XXX TEMP
        let opv_old = 
            if temp_perf_test {
                // Hammer: This code path is bogus; it's just here for some measurements I wanted
                cur.fill_empty();
                None
            } else {
                self.head.get_cursor(&mut cur, k.clone(), k_hash.clone())
            }
        ;
        let new_chk = match self.head {
            Chunks::Chunk(ref mut chk) => {
                chk.head = Some(chk.keys.len());
                chk.hash = my_hash(&(&chk.hash,&k_hash,&opv,&cur.paths));
                chk.keys.push(k);
                chk.vals.push(opv);
                chk.hashes.push(k_hash);
                chk.paths.push(cur.paths);
                None
            }
            Chunks::Link(ref _lnk) => {
                let mut chk = Chunk::new();
                chk.head = Some(0);
                chk.hash = my_hash(&(&k_hash,&opv,&cur.paths));
                chk.keys.push(k);
                chk.vals.push(opv);
                chk.hashes.push(k_hash);
                chk.paths.push(cur.paths);
                Some(chk)
            }
        };
        match new_chk {
            None    => (),
            Some(c) => { let _ = mem::replace(&mut self.head, Chunks::Chunk(c)); () },
        };
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
        let res = self.head.get_cursor( &mut cur, k, k_hash);
        match res {
            Some(opval) => opval,
            None => None,
        }
    }
}

#[test]
fn trie_opt_test () {
    use std::collections::HashMap;
    use rand::{Rng,thread_rng};
    use adapton::engine::{manage,name_of_usize};
    let mut rng = thread_rng();
    let numops = 10000;
    let numkeys = 100;
    let gauged = true;
    let gauge = 100;
    
    manage::init_dcg();
    
    let mut m = HashMap::new();
    let mut t = Trie::emp();
    
    for i in 0..numops {        
        let r1 : usize = rng.gen(); let r1 = r1 % numkeys;
        let r2 : usize = rng.gen(); let r2 = r2 % numkeys;
        let nm = if gauged && i % gauge == 0 { Some(name_of_usize(i)) } else { None };

        // Test random insertion
        if !(nm == None) { println!("=========\nname {:?}:", nm); };
        println!("insert #{:?}: key {:?} maps to {:?}", i, r1, r2);
        m.insert(r1, r2);
        t.put(r1, r2);
        match nm {
            Some(nm) => t.archive(nm),
            None => (),
        };

        // Test random lookup        
        let r3 : usize = rng.gen(); 
        let r3 = r3 % (numkeys * 2); // Look for non-existent keys with prob 0.5
        println!("lookup #{:?}: key {:?} maps to {:?}", i, r3, m.get(&r3));
        assert_eq!(m.get(&r3).map(|&n|n.clone()), t.get(r3));
    }
}


#[test]
fn trie_opt_tiny () {
    use adapton::engine::name_of_usize;
    let mut c = Trie::emp();
    c.put(1, 1);
    println!("{:?}\n", c);    
    c.put(2, 2);
    println!("{:?}\n", c);    
    c.put(3, 3);
    println!("{:?}\n", c);    
    c.put(4, 4);
    c.archive(name_of_usize(4));
    println!("{:?}\n", c);
    c.put(5, 5);
    println!("{:?}\n", c);
    c.put(6, 6);
    println!("{:?}\n", c);

    assert_eq!(c.get(0), None);
    assert_eq!(c.get(1), Some(1));
    assert_eq!(c.get(2), Some(2));
    assert_eq!(c.get(3), Some(3));
    assert_eq!(c.get(4), Some(4));
    assert_eq!(c.get(5), Some(5));
    assert_eq!(c.get(6), Some(6));
}

#[test]
fn trie_opt_small () {
    let mut c = Trie::emp();
    c.put(1, 1);
    c.put(2, 2);
    c.put(3, 3);
    c.put(4, 4);
    c.put(5, 5);
    c.put(6, 6);

    assert_eq!(c.get(0), None);
    assert_eq!(c.get(1), Some(1));
    assert_eq!(c.get(2), Some(2));
    assert_eq!(c.get(3), Some(3));
    assert_eq!(c.get(4), Some(4));
    assert_eq!(c.get(5), Some(5));
    assert_eq!(c.get(6), Some(6));

    c.put(11, 11);
    c.put(12, 12);
    c.put(13, 13);
    c.put(14, 14);
    c.put(15, 15);
    c.put(16, 16);

    assert_eq!(c.get(0), None);
    assert_eq!(c.get(1), Some(1));
    assert_eq!(c.get(2), Some(2));
    assert_eq!(c.get(3), Some(3));
    assert_eq!(c.get(4), Some(4));
    assert_eq!(c.get(5), Some(5));
    assert_eq!(c.get(6), Some(6));
    
    assert_eq!(c.get(7),  None);
    assert_eq!(c.get(8),  None);
    assert_eq!(c.get(9),  None);
    assert_eq!(c.get(10), None);

    assert_eq!(c.get(11), Some(11));
    assert_eq!(c.get(12), Some(12));
    assert_eq!(c.get(13), Some(13));
    assert_eq!(c.get(14), Some(14));
    assert_eq!(c.get(15), Some(15));
    assert_eq!(c.get(16), Some(16));

    assert_eq!(c.get(17), None);
    assert_eq!(c.get(18), None);
    assert_eq!(c.get(19), None);
    assert_eq!(c.get(20), None);

    c.put(21, 21);
    c.put(22, 22);
    c.put(23, 23);
    c.put(24, 24);
    c.put(25, 25);
    c.put(26, 26);

    assert_eq!(c.get(0), None);
    assert_eq!(c.get(1), Some(1));
    assert_eq!(c.get(2), Some(2));
    assert_eq!(c.get(3), Some(3));
    assert_eq!(c.get(4), Some(4));
    assert_eq!(c.get(5), Some(5));
    assert_eq!(c.get(6), Some(6));
    
    assert_eq!(c.get(7),  None);
    assert_eq!(c.get(8),  None);
    assert_eq!(c.get(9),  None);
    assert_eq!(c.get(10), None);

    assert_eq!(c.get(11), Some(11));
    assert_eq!(c.get(12), Some(12));
    assert_eq!(c.get(13), Some(13));
    assert_eq!(c.get(14), Some(14));
    assert_eq!(c.get(15), Some(15));
    assert_eq!(c.get(16), Some(16));

    assert_eq!(c.get(17), None);
    assert_eq!(c.get(18), None);
    assert_eq!(c.get(19), None);
    assert_eq!(c.get(20), None);

    assert_eq!(c.get(21), Some(21));
    assert_eq!(c.get(22), Some(22));
    assert_eq!(c.get(23), Some(23));
    assert_eq!(c.get(24), Some(24));
    assert_eq!(c.get(25), Some(25));
    assert_eq!(c.get(26), Some(26));

    assert_eq!(c.get(27), None);
    assert_eq!(c.get(28), None);
    assert_eq!(c.get(29), None);
    assert_eq!(c.get(30), None);
}
