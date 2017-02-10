// TODO: Figure out what to do with RNG's to avoid this
use std::marker::PhantomData;

use eval::*;

// builds a sequence from scratch, 
pub struct SizedSeq<E: Eval,G:ItemGen<E>> {
	pub size: usize, pub params: Params, pub item_gen: G, phantom: PhantomData<E>,
}
impl<D:InitSeq<G>,G:ItemGen<D::Item>> Creator<Duration,D> for SizedSeq<D::Item,G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		// we could time this and return it as overhead time
		D::init(&self.params, &self.item_gen, rng)
	}
}

pub struct SingleAppend;
impl<D: EditAppend> Editor<Duration,D> for SingleAppend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		// we could time this and return it as overhead time
		data.append(1, rng)
	}
}

/// Appends multiple elements
pub struct BatchAppend(pub usize);
impl<D: EditAppend> Editor<Duration,D> for BatchAppend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		// we could time this and return it as overhead time
		data.append(self.0, rng)
	}
}

pub struct FindMax;
impl<D: CompMax> Computor<Duration,D> for FindMax {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		// we could time this and return it as overhead time
		let (time,_answer) = data.seq_max(rng);
		time
	}
}

