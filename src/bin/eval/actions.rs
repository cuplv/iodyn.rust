// TODO: Figure out what to do with RNG's to avoid this
use std::marker::PhantomData;

use eval::*;

// builds a sequence from scratch, 
pub struct SizedSeq<E: Eval,G:ItemGen<E>> {
	pub size: usize, pub params: Params, pub item_gen: G, pub phantom: PhantomData<E>,
}
impl<D:InitSeq<G>,G:ItemGen<D::Item>> Creator<Duration,D> for SizedSeq<D::Item,G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::init(&self.params, &self.item_gen, rng)
	}
}

/// Action to add an element at the end of a collection
#[allow(unused)]
pub struct SingleAppend;
impl<D: EditAppend> Editor<Duration,D> for SingleAppend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.append(1, rng)
	}
}

/// Add multiple elements to the end of a collection
#[allow(unused)]
pub struct BatchAppend(pub usize);
impl<D: EditAppend> Editor<Duration,D> for BatchAppend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.append(self.0, rng)
	}
}

/// Add multiple elements to the end of a collection
#[allow(unused)]
pub struct BatchInsert(pub usize);
impl<D: EditInsert> Editor<Duration,D> for BatchInsert {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.insert(self.0, rng)
	}
}

/// Extends the collection as if it were being initialized,
/// that is, with init params rather than emulating user edits
#[allow(unused)]
pub struct BatchExtend(pub usize);
impl<D: EditExtend> Editor<Duration,D> for BatchExtend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.extend(self.0, rng)
	}
}

pub struct FindMax;
impl<D: CompMax> Computor<Duration,D> for FindMax {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.seq_max(rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
impl<D: CompMax> Computor<(Duration,D::Target),D> for FindMax {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Duration,D::Target) {
		data.seq_max(rng)
	}
}

