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
pub mod finite_map;

// Two forms of tries. They work, but performance needs improvement
#[doc(hidden)]
pub mod skiplist;
#[doc(hidden)]
pub mod trie;

/// Gauged Incremental Raz with element counts
pub type IRaz<E> = raz::Raz<E,raz_meta::Count>;
pub type Giraz<E> = raz::Raz<E,raz_meta::Count>;
/// Unfocused `IRaz`
pub type IRazTree<E> = raz::RazTree<E,raz_meta::Count>;
pub type GirazTree<E> = raz::RazTree<E,raz_meta::Count>;
/// Cross between vector and persistent stack
pub type ArchiveStack<E> = archive_stack::AStack<E,()>;

///level generator for incremental structures
pub fn inc_level() -> u32 {
  level_tree::gen_branch_level(&mut rand::thread_rng())
}
