IODyn: Collections for *Dynamic Input and Output*
==================================================

*IODyn* is collections library for programs that use [Adapton], a
general-purpose framework for incremental computing.

IODyn consists of collections for sequences, finite maps, sets and graphs.

Sequences
-------------
- Random Access Zipper (RAZ): Sequence as a zipper, with a cursor for local edits, local navigation, and global navigation (via an associated _level tree_ representation)
- Level tree: Sequence as a balanced tree; efficient global navigation, e.g., to an offset, to either end (first or last), or based on user-defined navigation data.
- Stack (last in first out): push, pop

Finite Maps, Sets
------------------
- Skip list: put, get, remove

In progress
============
- Queue (first in first out): push, pop
- Trie (persistent sets): put, get, remove, union, intersect
- Directed graph: XXX
- Undirected graph: XXX