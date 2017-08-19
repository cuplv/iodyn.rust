IODyn: Collections for **Dynamic Input and Output**
====================================================

**IODyn** is collections library for programs that use
[Adapton](http://rust.adapton.org), a general-purpose framework for
incremental computing.

IODyn consists of incremental, Adapton-based collections for sequences, finite maps, sets and graphs.

# Done: 

## Sequences

- **Random Access Zipper (RAZ)**: Sequence as a zipper, with a cursor for local edits, local navigation, and global navigation (via an associated _level tree_ representation)
- **Level tree**: Sequence as a balanced tree; efficient global navigation, e.g., to an offset, to either end (first or last), or based on user-defined navigation data.
- **Stack** (last in first out): push, pop

# In progress

## Finite maps and sets:
See https://github.com/cuplv/iodyn.rust/issues/20 for details

## More:
- Queue (first in first out): push, pop
- Trie (persistent sets): put, get, remove, union, intersect
- Directed graph: XXX
- Undirected graph: XXX
