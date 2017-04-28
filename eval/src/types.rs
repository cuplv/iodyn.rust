use std::ops::Add;
use rand::{Rng,Rand};
use std::hash::{Hash, Hasher};

#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct GenSmall(pub usize);
impl Rand for GenSmall{
  fn rand<R: Rng>(rng: &mut R) -> Self {
    GenSmall(rng.gen::<usize>() % 100)
  }
}
impl Add for GenSmall{
  type Output = GenSmall;
  fn add(self, rhs: Self) -> Self::Output {
    GenSmall(self.0 + rhs.0)
  }
}
impl From<usize> for GenSmall {
  fn from(num: usize) -> Self {
    GenSmall(num)
  }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,PartialOrd,Ord)]
pub struct Gen10k(pub usize);
impl Rand for Gen10k{
  fn rand<R: Rng>(rng: &mut R) -> Self {
    Gen10k(rng.gen::<usize>() % 10_000)
  }
}
impl Add for Gen10k{
  type Output = Gen10k;
  fn add(self, rhs: Self) -> Self::Output {
    Gen10k(self.0 + rhs.0)
  }
}



#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct Elm(pub usize);

const HASH_LOOP_COUNT: usize = 100;
impl Hash for Elm {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for _ in 0..HASH_LOOP_COUNT {
            self.0.hash(state);
        }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct GenSetElm(pub Elm);
impl Rand for GenSetElm{
  fn rand<R: Rng>(rng: &mut R) -> Self {
    GenSetElm(Elm(rng.gen::<usize>()))
  }
}
impl Add for GenSetElm{
  type Output = GenSetElm;
  fn add(self, rhs: Self) -> Self::Output {
      unreachable!()
  }
}
impl From<usize> for GenSetElm {
  fn from(num: usize) -> Self {
    GenSetElm(Elm(num))
  }
}
