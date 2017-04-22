use std::ops::Add;
use rand::{Rng,Rand};
use std::hash::{Hash, Hasher};

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
pub struct GenSmall(pub Elm);
impl Rand for GenSmall{
  fn rand<R: Rng>(rng: &mut R) -> Self {
    GenSmall(Elm(rng.gen::<usize>()))
  }
}
impl Add for GenSmall{
  type Output = GenSmall;
  fn add(self, rhs: Self) -> Self::Output {
    //GenSmall(Elm(self.0.0 + rhs.0.0))
      panic!("")
  }
}
impl From<usize> for GenSmall {
  fn from(num: usize) -> Self {
    GenSmall(Elm(num))
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

