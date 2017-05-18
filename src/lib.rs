//! A collection of incremental data structures with dynamic input
//! and output.
//!
//! Many of the structures have an exposed mutable head for fast
//! updates, and an `archive` function to move the current head
//! past a pointer, defining subsequences. This pointer is
//! mutable, so that the changes can propagate to later
//! computations.
//!
//! Fold and Map type computations over a level tree or a raz tree
//! will be memoized, such that rerunning the computation after a
//! change will be much faster than running from scratch.
//!
//! These collections are partially mutable and partially
//! persistent (shared data). However, the authors have
//! concentrated on incremental features, so partial sharing is not
//! well implemented. Data archived without names will be fully
//! persistent so that changes to cloned data will not affect the
//! original. Data archived with names should be treated as a mutable
//! structure, with changes to cloned data affecting the original. These
//! affects will probably not be consistent. Editing a mutable structure
//! within a namespace (`adapton::engine::ns`) should produce a version
//! whose edits do not affect the original, but this has not been
//! thoroughly tested.

extern crate rand;
#[macro_use] extern crate adapton;

<<<<<<< HEAD
/// Early work on traits for zippers
///
pub mod zip;                // trait for persistent zips
/// Persistent stack, modified slightly from "Learning Rust With Entirely Too Many Linked Lists"
///
pub mod stack;              // persistent stack
/// Early work on traits for the raz
///
pub mod seqzip;             // traits for persistent raz
pub mod persist_raz;        // monolithic single-item persistent raz
pub mod trees;              // traits for the various forms of trees
pub mod level_tree;         // persistent tree
pub mod tree_cursor;        // splittable cursor over tree (uses level_tree)
pub mod archive_stack;      // more complex stack (uses stack)
pub mod gauged_raz;         // raz of vectors using tree_cursor (uses archive_stack and tree_cursor)
// temp for incremental use
pub mod inc_level_tree;
pub mod inc_tree_cursor;
pub mod inc_gauged_raz;
pub mod finite_map;
pub mod inc_gauged_trie;

/// Persistent Raz - original design, simple but works
pub type PRaz<E> = persist_raz::Raz<E>;
/// Unfocused `PRaz`
pub type PRazTree<E> = persist_raz::RazSeq<E>;
/// Raz - Sequence editing. Vectorized leaves, manualy defined
pub type Raz<E> = gauged_raz::Raz<trees::NegBin,E>;
/// Unfocused `Raz`
pub type RazTree<E> = gauged_raz::RazTree<trees::NegBin,E>;
/// Incremental Raz - Experimental for use with Adapton
pub type IRaz<E> = inc_gauged_raz::Raz<E>;
=======
#[doc(hidden)]
pub mod trees;          // old work, but want to reincorporate the Level trait into current Raz
pub mod memo;           // Conversion function traits
pub mod stack;          // Cons-list
pub mod archive_stack;  // Sequences with subsequence vectors and metadata
pub mod level_tree;     // generic tree with cannonical structure, basis for incremental functions
pub mod tree_cursor;    // interface for traversing a level tree
pub mod raz;            // Gauged Incremental Random Access Zipper
pub mod raz_meta;       // Generic interface and concrete versions of metadata for searching the Raz
pub mod raz_based;      // Some simple structs based on the Raz

// Two forms of tries. They work, but performance needs improvement
#[doc(hidden)]
pub mod skiplist;
#[doc(hidden)]
pub mod trie;

/// Gauged Incremental Raz with element counts
pub type IRaz<E> = raz::Raz<E,raz_meta::Count>;
>>>>>>> f1bbd50ebc4562c7115fa6b30d0609dd3f61dfa5
/// Unfocused `IRaz`
pub type IRazTree<E> = raz::RazTree<E,raz_meta::Count>;
/// Cross between vector and persistent stack
pub type ArchiveStack<E> = archive_stack::AStack<E,()>;

///level generator for incremental structures
pub fn inc_level() -> u32 {
  level_tree::gen_branch_level(&mut rand::thread_rng())
}
