//! Incremental, high-gauge finite maps, for use by the Archivist in Adapton.
//!
//! Conceptually, these finite maps are tries.
//! Concretely, they consist of skip-list-like structures.

use std::rc::Rc;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash};
use adapton::engine::{cell,force,Art,Name,name_pair,name_of_usize};
use adapton::macros::{my_hash};

/// A hash value -- We define a custom Debug impl for this type.
#[derive(Clone,Hash,Eq,PartialEq)]
struct HashVal(usize);

/// A contiguous block of skiplist paths/keys/values
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Path<K,V> {
    //name:   Name,  // TODO-Soon: Want/need this here?
    //cntr:   usize, // TODO-Soon: Want/need this here?
    hash:   HashVal,
    kvs:    Vec<(K,Option<V>)>,
    paths:  Vec<Option<Art<Rc<Path<K,V>>>>>,
}

/// The gauged, incremental Skiplist.  This structure maintains the head
/// of the skip list, a name and a counter.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
pub struct Skiplist<K,V> {
    path_len: usize,
    name: Name,
    cntr: usize,
    head: Option<Art<Rc<Path<K,V>>>>,
}

/// Cursor state for traversing the skiplist
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Cursor<K,V> {
    /// Bit/path-position index; Invariant: increases monotonically
    bit_idx: usize,
    /// Invariant: `len(paths) ==  bit_idx`
    paths: Vec<Option<Art<Rc<Path<K,V>>>>>,
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

impl fmt::Debug for HashVal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
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

/// Invariant parameters to the observation: The key bits for which we
/// are looking, including how many remain.
struct ObsParam {
    /// Total number of bits in a full path.
    path_len:usize,
    /// Full list of bits for the search key.
    key_bits:usize,
    /// Which bit is next in our comparison, in interval [0,path_len).
    key_biti:usize,
}
struct ObsRes<K,V> {
    /// The paths that we observe; this is some sub-interval (a
    /// 'slice'?) of the full vector of paths in the observed `Path`.
    /// Its length depends on to what extent the search-key bits match
    /// those of the observed path.  There are two mutually-exclusive
    /// cases, each using an optional field below ('next' vs 'kvs').
    paths: Vec<Option<Art<Rc<Path<K,V>>>>>,
    /// Case 1: The key_bits do _not_ fully match those of the
    /// observed `Path`; the trampoline uses this `Art`, observing it
    /// next.
    next:  Option<Art<Rc<Path<K,V>>>>,
    /// Case 2: The key_bits _do_ fully match those of the observed
    /// `Path`; in this case, we also observe the key-value pairs.
    kvs:   Option<Vec<(K,Option<V>)>>,
}

/// This is the 'generic' signature for an observation function; it
/// receives the art, its full content of type `T` and returns some
/// partial view of this content, of type `S`.
// fn generic_observe<T,S> (art: &Art<T>, full_content:T) -> S {  unimplemented!() }


fn build_path_step<K:Clone,V:Clone>
    (art:    &Art<Rc<Path<K,V>>>,
     path:   &Rc<Path<K,V>>,
     params: ObsParam) -> ObsRes<K,V>
{
    // Mutable copies of these bit strings
    let mut key_bits = params.key_bits;
    let mut hsh_bits = path.hash.0.clone();    
    let mut cur_paths = vec![];
    
    // Discard bits that we've already "traversed"
    key_bits >>= params.key_biti;
    hsh_bits >>= params.key_biti;
    
    if hsh_bits == key_bits {
        for i in params.key_biti..params.path_len { 
            cur_paths.push(path.paths.get(i).unwrap().clone()) 
        };
        return ObsRes{
            paths:cur_paths,
            next: None,
            kvs: Some(path.kvs.clone())
        }
    } else {    
        let start_idx = params.key_biti;
        'matching_bits: 
        for i in start_idx..params.path_len {
            let oap : &Option<Art<Rc<Path<K,V>>>> = path.paths.get(i).unwrap();
            if (key_bits & 0x1) == (hsh_bits & 0x1) {
                cur_paths.push(oap.clone());
                key_bits >>= 1;
                hsh_bits >>= 1;
                continue 'matching_bits;
            } else {
                cur_paths.push(Some(art.clone()));
                match * path.paths.get(i).unwrap() {
                    None => {
                        for _ in i..params.path_len {
                            cur_paths.push(None)
                        };
                        return ObsRes{
                            paths:cur_paths,
                            next: None,
                            kvs: None
                        }
                    }
                    Some(ref a) => {
                        return ObsRes{
                            paths:cur_paths,
                            next: Some(a.clone()),
                            kvs: None
                        }
                    }
                }
            }
        };
        unreachable!("no more bits; this shouldn't happen")
    }
}

fn build_path_rec
    <K:'static+Hash+Eq+Debug+Clone,
     V:'static+Hash+Eq+Debug+Clone> 
    (path:&Rc<Path<K,V>>,
     path_len:usize,
     cur_art:Option<Art<Rc<Path<K,V>>>>,
     cur:&mut Cursor<K,V>,
     key_hash:HashVal) -> Option<Vec<(K,Option<V>)>> 
{
    panic!("TODO-Next")        
}



impl<K:'static+Hash+Eq+Debug+Clone,
     V:'static+Hash+Eq+Debug+Clone> Path<K,V> {
   
    fn build_path
        (&self,                    
         path_len:usize,
         cur_art:Option<Art<Rc<Path<K,V>>>>, 
         cur:&mut Cursor<K,V>, 
         key_hash:HashVal) -> Option<Vec<(K,Option<V>)>>
    {
        // Mutable copies of these bit strings
        let mut key_bits = key_hash.0;
        let mut hsh_bits = self.hash.0.clone();
        
        // Discard bits that we've already "traversed"
        key_bits >>= cur.bit_idx;
        hsh_bits >>= cur.bit_idx;

        if hsh_bits == key_bits {
            for i in cur.bit_idx..path_len { 
                cur.paths.push(self.paths.get(i).unwrap().clone()) 
            };
            return Some(self.kvs.clone());
        } else {    
            let start_idx = cur.bit_idx;
            'matching_bits: 
            for i in start_idx..path_len {
                let oap : &Option<Art<Rc<Path<K,V>>>> = self.paths.get(i).unwrap();
                if (key_bits & 0x1) == (hsh_bits & 0x1) {
                    cur.paths.push(oap.clone());
                    key_bits >>= 1;
                    hsh_bits >>= 1;
                    continue 'matching_bits;
                } else {
                    cur.bit_idx = i+1;
                    cur.paths.push(cur_art);
                    match * self.paths.get(i).unwrap() {
                        None => {
                            cur.fill_empty(path_len);
                            return None
                        }
                        Some(ref a) => {
                            return (force(a)).build_path(path_len, Some(a.clone()), cur, key_hash)
                        }
                    }
                }
            };
            unreachable!("no more bits; this shouldn't happen")
        }
    }       
}


/// Abstract, finite map interface implemented by the incremental Skiplist.
pub trait FinMap<K,V>
{
    /// An empty mapping; all keys map to `None`.
    fn emp(path_len:usize, nm:Name) -> Self;
    /// Update the Name in the head of the Skiplist
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
            head:None,
        }
    }
   
    fn archive(&mut self, n:Name) {
        self.name = n;
        self.cntr = 0;
    }

    fn ext(&mut self, k:K, opv:Option<V>) -> Option<V> {
        let mask    = make_mask(self.path_len);
        let k_hash  = HashVal((my_hash(&k) as usize) & mask);
        let mut cur = Cursor::new();
        let old_kvs = match self.head {
            None => None,
            Some(ref path) =>
                force(path).build_path(self.path_len, Some(path.clone()), &mut cur, k_hash.clone()),
        };
        let mut new_kvs = vec![];
        let mut opv_old = None;
        match old_kvs {
            Some(kvs) => {
                let mut found_key = false;
                for (k0,opv0) in kvs.into_iter() {
                    if &k == &k0 {
                        new_kvs.push((k0, opv.clone()));
                        opv_old = Some(opv0);
                        found_key = true;
                    } else {
                        new_kvs.push((k0, opv0));
                    }
                };
                if !found_key { 
                    new_kvs.push((k, opv)) 
                };
            },
            None => {
                cur.fill_empty(self.path_len);
                new_kvs.push((k, opv))
            }
        };
        let new_path = Path{
            hash:  k_hash,
            kvs:   new_kvs,
            paths: cur.paths,
        };
        self.head = 
            Some(cell(name_pair(self.name.clone(), 
                                name_of_usize(self.cntr)), 
                      Rc::new(new_path)));
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
        let mask    = make_mask(self.path_len);
        let k_hash  = HashVal((my_hash(&k) as usize) & mask);
        let mut cur = Cursor::new();
        let res = match self.head {
            None => None,
            Some(ref path) => force(path).build_path(self.path_len, None, &mut cur, k_hash),
        };            
        match res {
            Some(kvs) => {
                for (k0,opval) in kvs.into_iter() {
                    if &k == &k0 { return opval }
                };
                return None
            },
            None => None,
        }
    }
}

#[test]
fn skiplist_vs_hashmap () {
    use std::collections::HashMap;
    use rand::{Rng,thread_rng};
    use adapton::engine::{manage,name_of_usize,name_unit};
    let mut rng = thread_rng();
    let numops = 10000;
    let numkeys = 100;
    let gauged = true;
    let gauge = 100;
    
    manage::init_dcg();
    
    let mut m = HashMap::new();
    let mut t = Skiplist::emp(16,name_unit());
    
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
fn skiplist_tiny () {
    use adapton::engine::{name_unit, name_of_usize};
    let mut c = Skiplist::emp(8, name_unit());
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
