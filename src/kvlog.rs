// author: matthew.hammer@colorado.edu
/*!
**Key-value log**, a sequence of key-value accesses _and updates_.

# Key value log

A key-value log records a (totally-ordered) sequential history of
associations between keys and values.  At any point, the user can
append a new key-value association (function `put`) and access the most
recent value, if any, associated with a given key (function `get`).

For efficiency (to tune Adapton's overhead), we store the log in a chunked
representation, permitting us to amortize each Adapton-level operation
over _many_ key-value log operations, which form chunks.  The
programmer controls where/when the log breaks into separate chunks by
periodically calling `archive`.

## Interface overview

Abstract type `Log` represents a chunky key-value log:

- function `emp` produces the empty `Log`
- function `get` accesses the latest value for a given key
- function `put` updates the value for the given key
- function `archive` controls the representation's chunking, including the role of
   persistence and Adapton-level amortization.

See [this listing](#example-0) for a small example.

## Implementation discussion

We give a discussion of implementation details.

### Chunky representation

The chunky log representation amortizes Adapton operations over many
`put`/`get` operations.

- function `put` does not require any Adapton operations (it only
affects the current chunk), and

- function `get` uses Adapton operations in some cases, but not all:

  - `get` does not require Adapton observations for keys bound in the
    _current chunk_.

  - To resolve keys from _earlier chunks_, `get` observes `Art`s holding
    earlier chunks by using the chunks' _kv tables_, explained in detail
    below.  The expected number of such operations is related to the
    [expected fanout](#expected-fanout).

  - Transparently, `get` caches observations in the current chunk, to
    permit efficient `get` operations for any frequently-accessed keys.

- Function `archive` does not affect the key-value association.
Instead, it advances the chunked representation of the log by
completing the current chunk, and starting a new, empty chunk.  Behind
the scenes, it "completes" the chunk by computing the chunk's kv
table, and it stores the chunk in a single Adapton `Art` cell.

### Chunky list, chunky sequences

Collectively, the log's chunks form a linked list (via a `prev` field
in each chunk), permitting us to transform the log to and from other
sequence-based structures, including a RAZ, which we can edit
effectively in a uniformly random way.

### Chunky trie, via "kv tables"

In contrast to a standard list/sequence representation, the log also
forms a ("chunky") trie structure.  In particular, each completed
chunk holds its key-value pairs in an immutable, persistent "kv
table" that references earlier chunks with similar key hashes.

Conceptually, these kv tables represent a trie of the keys' hashes,
providing sub-linear time access to the log's most recent chunk to
update a given key of interest. In this sense, the log uses the
representation of a _chunky_ hash trie, or a trie indexed by the
hashes of keys, but allocated and addressed in a "chunky" way.

Concretely, the kv table of each chunk consists of a reference
counted hash table whose expected size the same as the _expected chunk
size_ (the expected number of key-value pairs between each _name_),
multiplied by the _expected address length_.

## Expected address length

As the number of distinct keys varies, the size of the conceptual trie
varies.  In expectation, with more keys, we require more hash bits to
distinguish any two of them.  For workloads with large numbers of
distinct keys (where one chunk is insufficient to hold all of the
keys' current values), we use "jump bits" to locate keys from earlier
chunks. In particular, for each jump bit, for each key, the kv table
of a chunk stores a "jump" (a pointer) to an earlier chunk.

The **expected address length** is the _expected number of bits
required to distinguish any two keys_.

Equivalently, the expected address length of a kv log is the expected
length of each non-singleton path in the conceptual trie.  A bit
string addresses a _singleton path_ when it forms the prefix of
exactly one key.  A (non-empty) non-singleton path consists of a bit
string that forms the prefix of two or more distinct keys.

### Bit string addresses for keys

We divide the hash of each key (the "key bits") into three parts:

- the "chunk bits" (least significant), e.g., 10 bits,
- the "jump bits" (next significant), e.g., the next 0--20 bits and
- the extra bits (least significant) that we do not use.

The number of "key bits" should suffice to make each distinct key have
a good chance of having a distinct hash.  In practice, we choose 64
bits; we detect and tolerate collisions, so this choice seems
sufficient.

Ideally, the number of "chunk bits" plus the number of "jump bits"
should be close to the [expected address
length](#expected-address-length) of the kv log.

**Examples**

- For 4096 distinct keys, we expect the address length to be 12 bits,
since 2^12 = 4096; if we set the chunk size to 1k, the number of chunk
and jump bits is 10 and 2, respectively.

- For 1M distinct keys and 1k chunks, we may want 10 jump bits, since
1M = 2^20 = 2^(10+10).

Generally, the number of "chunk bits" should be related to the
expected size of each chunk and when the number of keys is much larger
than this number, the "jump bits" should suffice to make lookups cheap
(requiring more bits, generally).  However, these jump bits come at a
cost when building and incrementally maintaining the kv log: Behind
the scenes, we store a "jump" (a pointer) for each each jump bit, for
each (unique) chunk key.

### Tuning costs and bit lengths

Ideally, the number of "jump bits" should augment the "chunk bits" to
uniquely identify each key (i.e., they should total the number of
expected address bits).  For instance, for 2^20 = 1M distinct keys, we
may choose to have 10 "chunk bits" and 10 "jump bits", for 20 bits
total.  In this case, we choose the "jump bits" such that the number
of extra bits is zero.

In practice, computing and storing jump pointers has an upfront cost.
To tune this, we chould set the number of "jump bits" to be fewer
than this ideal choice, and the "chunk bits" plus "jump bits" together
may not assign distinct bits to distinct key bit strings.

Varying this balance presents a trade off: the imprecision of having
fewer bits for locating keys makes _building_ the log's kv tables
cheaper, but each _key lookup_ later on becomes (potentially) less
precise and more expensive, in expectation.

In the limit, the number of "jump bits" is zero, and only the "chunk
bits" are used to distinguish keys, where the number of keys could be
`O(n)`, where `n` is much larger than the number keys of a single
chunk.  In this limit case, jump sequences that search for a key
contain _many_ distinct keys (`O(n)` of them, in expectation), and
consequently, key lookups take `O(n)` worst-case time.

### Fanout metrics

In addition to the time and space costs mentioned above, we wish to
analytically (_combinatorially_) characterize the cost of Adapton's DCG
representation, which caches the construction of the kv log. To do so,
we introduce the following definitions:

- The **expected pointer fanout** of a chunk is the expected number
  of distinct **chunks** that each chunk _directly references_ in its kv
  table.

- The **expected dependency fanout** of a chunk is the expected
  number of distinct **chunks** that each chunk _directly observes_
  while
  - performing `get` operations on earlier chunks and
  - performing the `archive` operation, which computes the chunk's kv table, and its jump pointers.
  - (recall that `put` operations do not observe earlier chunks)

Conjecture: The expected dependency fanout is less than or equal to
the expected pointer fanout. (Proof: ?)

Question: How are these two fanout metrics related combinatorially?
(How are they related empirically?)

Question: For chunk size C and distinct keys K, and a uniform
distribution of operations over keys, what are the expected fan outs
of the log's chunks?

In particular, how is it related to the expected path length?

# Examples

## Example 0

```
extern crate adapton;
extern crate iodyn;
use adapton::engine::*;
use iodyn::kvlog::Log;

fn main () {
    let mut l = Log::emp();

    l.put(1, 1);
    l.put(2, 2);
    l.put(2, 3);
    l.put(2, 1);
    l.put(3, 3);
    l.put(4, 4);

    assert_eq!(l.get(0), None);
    assert_eq!(l.get(1), Some(1));
    assert_eq!(l.get(2), Some(1));
    assert_eq!(l.get(3), Some(3));
    assert_eq!(l.get(4), Some(4));
    assert_eq!(l.get(5), None);
    assert_eq!(l.get(6), None);

    l = l.archive(Some(name_of_usize(4)));
    l.put(5, 5);
    l.put(6, 6);
    l.put(2, 4);

    assert_eq!(l.get(0), None);
    assert_eq!(l.get(1), Some(1));
    assert_eq!(l.get(2), Some(4));
    assert_eq!(l.get(3), Some(3));
    assert_eq!(l.get(4), Some(4));
    assert_eq!(l.get(5), Some(5));
    assert_eq!(l.get(6), Some(6));
}
```

!*/

// ### Example

// TODO: Check calculations in example below (might be imprecise/wrong):

// For 2048 distinct keys, and chunk sizes of 1k, we expect the path
// length to be 1 bit, since 10 bits (1024 slots) only require 1
// additional bit to distinguish 2048 items. For 1M distinct keys and 1k
// chunks, the expected path length is 10 bits (10 chunk bits + 10 path
// bits = 20 bits total, which suffices for 1M items, in expectation).

//use std::fmt;
use std::rc::Rc;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::{HashMap,DefaultHasher};
use adapton::engine::*;
use trie2;

fn my_hash<T>(obj: T) -> u64
  where T: Hash
{
  let mut hasher = DefaultHasher::new();
  obj.hash(&mut hasher);
  hasher.finish()
}

const CHUNK_BITS : u32 = 0x4ff;

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub struct Log<K,V> ( Option<Chunk<K, V>> );

// log chunk
#[derive(Clone,PartialEq,Eq,Debug)]
struct Chunk<K, V> {
    here: Here<K, V>,
    prev: Option<Art<Chunk<K, V>>>,
    name: Option<Name>,
}
// The key-value pairs "here" consist of either a vector of tuples, or
// a pre-computed "kv table".
#[derive(Clone,PartialEq,Eq,Debug)]
enum Here<K,V> {
    Vec(Vec<(K, KeyBits, V)>),
    Table(Rc<KvTable<K, V>>)
}

// some bits of the key's full hash value, used to create a "kv table"
// (partially) mapping this hash prefix to a `Kv`.
#[derive(Clone,PartialEq,Eq,Debug,Hash)]
struct ChunkBits ( u32 );

// the key's full hash value, used to create a "kv table"
// (partially) mapping this hash prefix to a `Kv`.
#[derive(Clone,PartialEq,Eq,Debug,Hash)]
struct KeyBits ( u64 );

// map "chunk bits" to `Kv`s.
//
// For chunks of size 1k, choose the number of "chunk bits" to be ~10,
// since exp(2, 10) = 1024.  As the log grows, there can never be more
// than 1024 entries in the kv table of any chunk (by the pigeon
// hole principle), and these entries will always reflect the "most
// recent" jump paths, since the kv tables accumulate the jumps of
// all prior chunks, in order.
type KvTable<K,V> = HashMap<ChunkBits, Vec<Kv<K, V>>> ;

// a "kv" is a hash, a key-value pair (whose key has the given hash),
// and a collection of one or more "jump" pointers to previous chunks
// that have keys with related hash strings.
#[derive(Clone,PartialEq,Eq,Debug,Hash)]
struct Kv<K, V> {
    // Key
    key: K,
    // Value
    val: V,
    // bits consists of all of the hash bits of the key, including the
    // "chunk" bits and "jump" bits. if this full hash matches during a
    // key-value lookup, the lookup key's value is in the current
    // chunk, and no jump to another chunk is necessary. otherwise, if
    // the "chunk bits" match but the full bits do not, consult the
    // `jumps` field at the offset of first "jump bit" mismatch.
    bits: KeyBits,
    // for each "jump bit" in the Kv, we give an optional log chunk.
    // This chunk represents where to lookup next when the lookup bit
    // does not match a jump bit. if all jump bits match, but two keys
    // are distinct, then use `prev` to find earlier occurrences of
    // the same bit pattern.
    jumps: Vec<Option<Art<Chunk<K, V>>>>,
    // the previous chunk that contains a key with _the exact same_
    // chunk bits and jump bits. this field is needed when two distinct
    // keys have the same chunk+jump bits; it permits us to traverse
    // the chunks that contain both keys.
    prev: Option<Art<Chunk<K,V>>>,
}

// Adapton engine requires the Hash trait for data in Arts, but it's
// not used for nominal Arts, which should be the norm here.  This
// trait is only used for an Art that is named structurally (via the
// hash of its content).
impl<K:Hash,V:Hash> Hash for Chunk<K,V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.prev.hash(state);
        match self.here {
            Here::Vec(ref kvs) => kvs.hash(state),
            Here::Table(ref kvt) => {
                for (ref bits, ref kvs) in kvt.iter() {
                    bits.hash(state);
                    kvs.hash(state);
                }
            }
        }
    }
}

impl <K:'static+Hash+PartialEq+Eq+Clone+Debug,
      V:'static+Hash+PartialEq+Eq+Clone+Debug> Log<K,V> {

    fn key_bits(key: &K) -> KeyBits { KeyBits(my_hash(key)) }
    
    fn chunk_bits(bits:&KeyBits) -> ChunkBits {
        ChunkBits((bits.0 & (CHUNK_BITS as u64)) as u32)
    }

    fn build_path_rec
        (bits: KeyBits,
         chunk_art: Option<&Art<Chunk<K,V>>>, chunk: &Chunk<K,V>,
         jumps: &mut Vec<Option<Art<Chunk<K,V>>>>)
         -> Option<Art<Chunk<K,V>>>
    {
        match chunk.here {
            Here::Vec(ref kvs) => {
                for &(ref k, ref kbits, ref v) in kvs.iter().rev() {
                    if bits.0 == kbits.0 {
                        assert_eq!(chunk_art, None);
                        return None
                    }
                    else { continue }
                };
                // not found "here", so do a lookup in previous chunk
                match chunk.prev {
                    None => None,
                    Some(ref prev) => {
                        Self::build_path_rec(bits, Some(prev), &force(prev), jumps)
                    },
                }
            }
            Here::Table(ref kvt) => {
                match kvt.get(&Self::chunk_bits(&bits)) {
                    None => {
                        // the key does not exist
                        return None
                    }
                    Some(kvs) => {
                        // first, look for an exact KeyBits match among the Kvs
                        for kv in kvs.iter() {
                            // compare kv.bits with bits. How many of them match?
                            // assert invariant: at least depth (+1?) bits match?
                            if bits == kv.bits {
                                return chunk_art.map(|x|x.clone())
                            }
                        };
                        // no exact match for all KeyBits here.  So,
                        // we need to "jump" to another chunk.
                        let mut next_chunk_art : Option<&Art<Chunk<K,V>>> = None;
                        // to determine the next chunk, look for
                        // longest KeyBits match among the Kvs we have
                        // in `kvs`, and its jumps.
                        for kv in kvs.iter() {
                            // TODO look for longest match among the Kvs
                            // If found, extend jumps for each matching bit
                            unimplemented!()
                        };
                        // search in the next chunk, if any
                        match next_chunk_art {
                            None => None,
                            Some(chunk_art) => {
                                return Self::build_path_rec
                                    (bits, Some(chunk_art), &force(chunk_art), jumps)
                            }
                        }
                    }
                }
            }
        }
    }


    pub fn emp() -> Log<K,V> { Log(None) }

    pub fn get(&mut self, key: K) -> Option<V> {
        let mut do_put : bool = false ;
        let key_bits = Self::key_bits(&key);        
        let vo : Option<V> = match self.0 {
            None => None,
            Some(ref mut chunk) => {
                match chunk.here {
                    Here::Vec(ref mut kvs) => {
                        let mut vo = None ;
                        for &(ref k, ref bits, ref v) in kvs.iter().rev() {
                            if &key_bits == bits {
                                assert_eq!(&key, k);
                                vo = Some(v.clone());
                                break;
                            }
                        };
                        if vo == None { // key not found in vector. if
                            // we find it elsewhere in an earlier
                            // chunk, also put it in this chunk.
                            do_put = true;
                            match chunk.prev {
                                None => (),
                                Some(ref x) => {
                                    vo = Log(Some(force(x))).get(key.clone());                                    
                                }
                            }
                        }
                        vo
                    },
                    Here::Table(ref tab) => {
                        match tab.get(&Self::chunk_bits(&key_bits)) {
                            None => None,
                            Some(ref kvs) => {
                                let mut vo = None ;
                                for kv in kvs.iter() {
                                    if key_bits == kv.bits {
                                        assert_eq!(&key, &kv.key);
                                        vo = Some(kv.val.clone());
                                        break;
                                    }
                                };
                                if vo == None {
                                    // Now, we need to use one of the "jumps" (if any) to proceed
                                    unimplemented!()
                                };
                                vo
                            }
                        }
                    }
                }
            }
        };
        if do_put {
            // TODO-Someday: Store value options for keys (not merely
            // values), to also store the _absence_ of values.
            vo.map(|x|{self.put(key, x.clone()); x})
        } else {
            vo
        }
    }

    pub fn put(&mut self, key: K, val: V) {
        let key_bits = Self::key_bits(&key);
        match self.0 {
            None => self.0 = Some(Chunk{
                here:Here::Vec(vec![ (key, key_bits, val) ]),
                prev:None,
                name:None,
            }),
            Some(ref mut chunk) => match chunk.here {
                Here::Vec(ref mut kvs) => kvs.push((key, key_bits, val)),
                Here::Table(_) => unreachable!(),
            }};
    }

    pub fn archive(mut self, on:Option<Name>) -> Self {
        match self.0 {
            None => Log(Some(Chunk{
                here:Here::Vec(vec![]),
                prev:None,
                name:on,
            })),
            Some(mut chunk) => {
                let mut tab = HashMap::new();
                match chunk.here {
                    Here::Table(kvt) => {
                        // not possible, since an invariant is that
                        // the head of every log is a chunk with Vec
                        // representation.
                        unreachable!()
                    },
                    Here::Vec(kvs) => {
                        // initial kv table consists of most recent key bindings from _this chunk_:
                        for (k, kbits, v) in kvs.into_iter().rev() {
                            let chunk_bits = Self::chunk_bits(&kbits);
                            if ! tab.contains_key(&chunk_bits) {
                                tab.insert(chunk_bits.clone(), vec![] );
                            };
                            match tab.get_mut(&chunk_bits) {
                                None => unreachable!(), // impossible: just inserted an empty vector here.
                                Some(kvs) => {
                                    let mut jumps = Vec::new();
                                    let prev = match chunk.prev {
                                        None => None,
                                        Some(ref chunk_art) => {
                                            Self::build_path_rec
                                                (kbits.clone(), Some(chunk_art), &force(chunk_art), &mut jumps)
                                        }
                                    };
                                    kvs.push(Kv{key:k, val:v, bits:kbits, prev:prev, jumps:jumps})
                                }
                            }
                        };
                        // next, temporarily borrow chunk to follow
                        // prev pointer; update any "holes" in the kv
                        // table with kvs from the previous kv table.
                        {
                            match chunk.prev { 
                                None => (),
                                Some(ref a) => match get!(a).here {
                                    Here::Vec(_) => unreachable!(),
                                    Here::Table(ref prev_tab) => {
                                        for (bits, kvs) in prev_tab.iter() {
                                            if ! tab.contains_key(bits) {
                                                tab.insert(bits.clone(), kvs.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        };
                        // save the kv table we just computed,
                        // replacing the vector representation of the
                        // current chunk.
                        chunk.here = Here::Table(Rc::new(tab));
                        // The new/empty chunk consists of an empty
                        // vector; current chunk becomes the previous
                        // chunk of this new, empty head chunk.
                        Log(Some(Chunk{
                            here:Here::Vec(vec![]),
                            prev:Some(cell!([on.clone()]? chunk)),
                            name:on,
                        }))
                    }
                }
            }
        }
    }

    pub fn into_trie(self) -> trie2::Trie<K,V> {
        unimplemented!()
    }
}

/// Key-value log with linear operations (no references)
///
/// `LinLog` Demonstrates another API variation, where the log is
/// _moved_ by its operations, as a _"linear resource"_. The mutable
/// ref interface for `Log` suffices to implement this interface.
struct LinLog<K,V> (Log<K,V>);
impl <K:'static+Hash+PartialEq+Eq+Clone+Debug,
      V:'static+Hash+PartialEq+Eq+Clone+Debug> LinLog<K,V> {

    pub fn emp() -> LinLog<K,V> {
        LinLog(Log(None))
    }

    pub fn archive(mut self, on:Option<Name>) -> LinLog<K,V> {
        LinLog(self.0.archive(on))
    }

    pub fn put(mut self, key: K, val: V) -> LinLog<K,V> {
        self.0.put(key, val);
        self
    }

    pub fn get(mut self, key: K) -> (Option<V>, LinLog<K,V>) {
        let vo = self.0.get(key);
        (vo, self)
    }

    pub fn into_trie(mut self) -> trie2::Trie<K,V> {
        unimplemented!()
    }
}




#[test]
fn kvlog_tiny () {
    use adapton::engine::{name_unit, name_of_usize};
    let mut c = Log::emp();
    c.put(1, 1);
    println!("{:?}\n", c);
    c.put(2, 2);
    c.put(2, 3);
    println!("{:?}\n", c);
    c.put(2, 1);
    c.put(3, 3);
    println!("{:?}\n", c);
    c.put(4, 4);

    println!("lookup 1--6:\n");

    assert_eq!(c.get(0), None);
    assert_eq!(c.get(1), Some(1));
    assert_eq!(c.get(2), Some(1));
    assert_eq!(c.get(3), Some(3));
    assert_eq!(c.get(4), Some(4));
    assert_eq!(c.get(5), None);
    assert_eq!(c.get(6), None);

    println!("done.\n");

    c = c.archive(Some(name_of_usize(4)));
    println!("{:?}\n", c);
    c.put(5, 5);
    println!("{:?}\n", c);
    c.put(6, 6);
    println!("{:?}\n", c);

    assert_eq!(c.get(0), None);
    assert_eq!(c.get(1), Some(1));
    assert_eq!(c.get(2), Some(1));
    assert_eq!(c.get(3), Some(3));
    assert_eq!(c.get(4), Some(4));
    assert_eq!(c.get(5), Some(5));
    assert_eq!(c.get(6), Some(6));
}
