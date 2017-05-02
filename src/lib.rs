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

pub mod trees;              // traits for the various forms of trees
pub mod memo;
pub mod inc_stack;
pub mod inc_archive_stack;
pub mod inc_level_tree;
pub mod inc_tree_cursor;
pub mod inc_gauged_raz;
pub mod raz_meta;

pub mod skiplist;
pub mod trie;

/// Gauged Incremental Raz - Experimental for use with Adapton
pub type IRaz<E> = inc_gauged_raz::Raz<E,raz_meta::Count>;
/// Unfocused `IRaz`
pub type IRazTree<E> = inc_gauged_raz::RazTree<E,raz_meta::Count>;
/// Cross between vector and persistent stack
pub type ArchiveStack<E> = inc_archive_stack::AStack<E,()>;

///level generator for incremental structures
pub fn inc_level() -> u32 {
  inc_level_tree::gen_branch_level(&mut rand::thread_rng())
}
