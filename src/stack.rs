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
    elem: Rc<T>,
    next: Link<T>,
}

pub struct Iter<T> {
    next: Link<T>,
}


impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack { head: None }
    }

    pub fn is_empty(&self) -> bool {
        if let None = self.head { true } else { false }
    }

    pub fn push(&self, elem: Rc<T>) -> Stack<T> {
        Stack { head: Some(Rc::new(Node {
            elem: elem,
            next: self.head.clone(),
        }))}
    }

    pub fn peek(&self) -> Option<Rc<T>> {
        self.head.as_ref().map(|ref node| node.elem.clone())
    }

    pub fn pull(&self) -> Option<Stack<T>> {
        self.head.as_ref().map(|node| Stack { head: node.next.clone()})
    }

    pub fn iter(&self) -> Iter<T> {
        Iter { next: self.head.clone() }
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
// so we iterate
impl<T> Drop for Stack<T> {
  fn drop(&mut self) {
    let mut head = self.head.take();
    while let Some(node) = head {
      if let Ok(mut node) = Rc::try_unwrap(node) {
        head = node.next.take();
      } else {
        break;
      }
    }
  }
}

// derive will require the inner data be `Clone` for some reason
impl<T> Clone for Stack<T> {
    fn clone(&self) -> Self {
        Stack { head: self.head.clone() }
    }
}

impl<T> Iterator for Iter<T> {
    type Item = Rc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let (n,r) = match self.next { None => (None,None),
            Some(ref node) => (node.next.clone(),Some(node.elem.clone()))
        };
        self.next = n;
        r
    }
}

