// Modified from "Learning Rust With Entirely Too Many Linked Lists":
// http://cglab.ca/~abeinges/blah/too-many-lists/book/third-final.html

use std::rc::Rc;

// this layout is explained here: http://cglab.ca/~abeinges/blah/too-many-lists/book/first-layout.html
// keeps data in heap; uses `Option`s null pointer optimization

#[derive(Debug)]
pub struct Stack<T> {
    head: Link<T>,
}

type Link<T> = Option<Rc<Node<T>>>;

#[derive(Debug)]
struct Node<T> {
    elem: T,
    next: Link<T>,
}

pub struct Iter<'a, T:'a> {
  next: Option<&'a Node<T>>,
}


impl<T: Clone> Stack<T> {
    pub fn new() -> Self {
        Stack { head: None }
    }

    pub fn is_empty(&self) -> bool {
        if let None = self.head { true } else { false }
    }

    pub fn push(&self, elem: T) -> Stack<T> {
        Stack { head: Some(Rc::new(Node {
            elem: elem,
            next: self.head.clone(),
        }))}
    }

    pub fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|ref node| &node.elem)
    }

    pub fn pull(&self) -> Option<Stack<T>> {
        self.head.as_ref().map(|node| Stack { head: node.next.clone()})
    }

    pub fn iter(&self) -> Iter<T> {
        Iter { next: self.head.as_ref().map(|node| &**node) }
    }
}

impl<T: Clone> Stack<T> {
  pub fn rev(&self) -> Stack<T> {
    let mut outlist = Stack::new();
    for item in self.iter() {
      outlist = outlist.push(item.clone())
    }
    outlist
  }
}

// the default will recurse through this stack, increasing program's stack,
// so we iterate (but it's slightly slower)
// impl<T> Drop for Stack<T> {
//   fn drop(&mut self) {
//     let mut head = self.head.take();
//     while let Some(node) = head {
//       if let Ok(mut node) = Rc::try_unwrap(node) {
//         head = node.next.take();
//       } else {
//         break;
//       }
//     }
//   }
// }

// derive will require the inner data be `Clone` for some reason
impl<T: Clone> Clone for Stack<T> {
    fn clone(&self) -> Self {
        Stack { head: self.head.clone() }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
  type Item = &'a T;

  fn next(&mut self) -> Option<Self::Item> {
    self.next.map(|node| {
      self.next = node.next.as_ref().map(|node| &**node);
      &node.elem
    })
  }
}
