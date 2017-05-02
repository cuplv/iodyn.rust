//! Skip-lists, for representing finite maps.
//!
//! Suitable for the Archivist role in Adapton.
//!
// Matthew Hammer <Matthew.Hammer@Colorado.edu>

use std::rc::Rc;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash,Hasher};
use std::collections::hash_map::{DefaultHasher};
use adapton::engine::{cell,force_map,Art,Name,name_pair,name_of_usize};

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

/// A contiguous block of skiplist paths/keys/values
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct Path<K,V> {
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
            paths: vec![],
        }
    }
    fn fill_empty(&mut self, bits:usize) {
        for _ in self.paths.len()..bits {
            self.paths.push(None)
        }
    }
}

/// Invariant parameters to the observation: The key bits for which we
/// are looking, including how many remain.
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct ObsParam {
    /// Total number of bits in a full path.
    path_len:usize,
    /// Full list of bits for the search key.
    key_bits:usize,
    /// Which bit is next in our comparison, in interval [0,path_len).
    key_biti:usize,
}
#[derive(Debug,Clone,Hash,Eq,PartialEq)]
struct ObsRes<K,V> {
    /// The paths that we observe; this is some sub-interval of the
    /// full vector of paths in the observed `Path`.  Its length
    /// depends on to what extent the search-key bits match those of
    /// the observed path.  There are three mutually-exclusive cases,
    /// no match at all (both `next` and `kvs` are None), and two
    /// remaining cases, each using an optional field below ('next' vs
    /// 'kvs' being Some(_) and the other None).
    paths: Vec<Option<Art<Rc<Path<K,V>>>>>,
    /// Case 1: The key_bits do _not_ fully match those of the
    /// observed `Path`; the trampoline uses this `Art`, observing it
    /// next.
    next:  Option<Art<Rc<Path<K,V>>>>,
    /// Case 2: The key_bits _do_ fully match those of the observed
    /// `Path`; in this case, we also observe the key-value pairs.
    kvs:   Option<Vec<(K,Option<V>)>>,    
}

fn build_path_step<K:Clone,V:Clone>
    (art:    &Art<Rc<Path<K,V>>>,
     path:   Rc<Path<K,V>>,
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
        return ObsRes{ paths:cur_paths, next: None, kvs: Some(path.kvs.clone()) }
    } else {
        'matching_bits: 
        for i in params.key_biti..params.path_len {
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
                        for _ in (i+1)..params.path_len {
                            cur_paths.push(None)
                        };
                        return ObsRes{ paths:cur_paths, next: None, kvs: None }
                    }
                    Some(ref a) => {
                        return ObsRes{ paths:cur_paths, next: Some(a.clone()), kvs: None }
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
    (path_len:usize,
     key_hash:HashVal,
     cur_art:&Art<Rc<Path<K,V>>>,
     cursor:&mut Cursor<K,V>
    ) -> Option<Vec<(K,Option<V>)>> 
{    
    let key_bits : usize = key_hash.0;
    let key_biti : usize = cursor.paths.len();
    let mut res =
        force_map(&cur_art, move |art,path|
                  build_path_step (art, path, ObsParam{ path_len:path_len, key_bits:key_bits, key_biti:key_biti})) ;
    cursor.paths.append(&mut res.paths);
    match res.kvs {
        Some(kvs) => {
            // Case: Key's hash _is_ present; return all key-value pairs with matching hash
            assert_eq!(cursor.paths.len(), path_len);
            Some(kvs)
        },
        None => {
            match res.next {
                Some(ref next_art) =>
                    // Case: Key's hash _may or may not_ be present; need to search further to determine this question.
                    build_path_rec(path_len, key_hash, next_art, cursor),
                None => {
                    // Case: Key's hash _is not_ present:
                    assert_eq!(cursor.paths.len(), path_len);
                    return None
                }
            }
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
                build_path_rec(self.path_len, k_hash.clone(), path, &mut cur),
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
            Some(ref path) =>
                build_path_rec(self.path_len, k_hash, path, &mut cur),
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
    let gauge = 10;
    
    manage::init_dcg();
    
    let mut m = HashMap::new();
    let mut t = Skiplist::emp(8,name_unit());
    
    for i in 0..numops {        
        let r1 : usize = rng.gen(); let r1 = r1 % numkeys;
        let r2 : usize = rng.gen(); let r2 = r2 % numkeys;
        let nm = if gauged && i % gauge == 0 { Some(name_of_usize(i)) } else { None };

        // Test random insertion
        //if !(nm == None) { println!("=========\nname {:?}:", nm); };
        //println!("insert #{:?}: key {:?} maps to {:?}", i, r1, r2);
        m.insert(r1, r2);
        t.put(r1, r2);
        match nm {
            Some(nm) => t.archive(nm),
            None => (),
        };

        // Test random lookup        
        let r3 : usize = rng.gen(); 
        let r3 = r3 % (numkeys * 2); // Look for non-existent keys with prob 0.5
        //println!("lookup #{:?}: key {:?} maps to {:?}", i, r3, m.get(&r3));
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


