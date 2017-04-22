IODyn: Collections library for computing with _dynamic input and output_
========================================================================

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
- Trie: put, get, remove, union, intersect
