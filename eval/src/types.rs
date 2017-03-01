use std::ops::Add;
use rand::{Rng,Rand};

#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,PartialOrd,Ord)]
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

