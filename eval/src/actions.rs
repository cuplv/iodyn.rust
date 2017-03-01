use std::marker::PhantomData;
use std::rc::Rc;
use rand::{Rng,StdRng};
use time::Duration;
use primitives::*;

pub trait Creator<R,D> {
	fn create(&mut self, rnd: &mut StdRng) -> (R,D);
}
pub trait Editor<R,D> {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (R,D);
}
pub trait Computor<R,D> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> R;
}

/// Test framework
pub trait Testor<R> {
	fn test(&mut self, rng: &mut StdRng) -> R;
}

// builds a sequence from scratch, 
#[derive(Clone)]
pub struct IncrementalInit<G:Rng> {
	pub size: usize,
	pub unitgauge: usize,
	pub namegauge: usize,
	pub coord: G,
}
impl<D:CreateInc<G>,G:Rng>
Creator<Duration,D> for IncrementalInit<G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::inc_init(self.size, self.unitgauge, self.namegauge, &self.coord, rng)
	}
}

/// Action to add an element at the end of a collection
pub struct SingleAppend;
impl<D: EditAppend>
Editor<Duration,D> for SingleAppend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.append(1, rng)
	}
}

/// Add multiple elements to the end of a collection
pub struct BatchAppend(pub usize);
impl<D: EditAppend>
Editor<Duration,D> for BatchAppend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.append(self.0, rng)
	}
}

/// Add multiple elements to the end of a collection
pub struct BatchInsert(pub usize);
impl<D: EditInsert>
Editor<Duration,D> for BatchInsert {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.insert(self.0, rng)
	}
}

/// Extends the collection as if it were being initialized,
/// that is, with init params rather than emulating user edits
pub struct BatchExtend(pub usize);
impl<D: EditExtend>
Editor<Duration,D> for BatchExtend {
	fn edit(&mut self, data: D, rng: &mut StdRng) -> (Duration,D) {
		data.extend(self.0, rng)
	}
}

// TODO: rewrite these using treefold
pub struct FindMax;
impl<D: CompMax>
Computor<Duration,D> for FindMax {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.comp_max(rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
impl<D: CompMax>
Computor<(Duration,D::Target),D> for FindMax {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Duration,D::Target) {
		data.comp_max(rng)
	}
}

pub struct TreeFold<E,O,I:Fn(&E)->O,B:Fn(O,O)->O>(Rc<I>,Rc<B>,PhantomData<E>,PhantomData<O>);
impl<E,O,I:Fn(&E)->O,B:Fn(O,O)->O> TreeFold<E,O,I,B> { pub fn new(init:I,bin:B) -> Self {TreeFold(Rc::new(init),Rc::new(bin),PhantomData,PhantomData)}}
impl<E,O,I:Fn(&E)->O,B:Fn(O,O)->O,D: CompTreeFold<E,O,I,B>>
Computor<Duration,D> for TreeFold<E,O,I,B> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time, answer) = data.comp_tfold(self.0.clone(),self.1.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}

pub struct Mapper<I,O,F:Fn(&I)->O>(Rc<F>,PhantomData<I>,PhantomData<O>);
impl<I,O,F:Fn(&I)->O> Mapper<I,O,F> { pub fn new(f:F) -> Self {Mapper(Rc::new(f),PhantomData,PhantomData)}}
impl<I,O,F:Fn(&I)->O,D:CompMap<I,O,F>>
Computor<Duration,D> for Mapper<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.comp_map(self.0.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}

pub struct Folder<I,O:Clone,F:Fn(O,&I)->O>(O,Rc<F>,PhantomData<I>,PhantomData<O>);
impl<I,O:Clone,F:Fn(O,&I)->O> Folder<I,O,F> { pub fn new(a:O,f:F) -> Self {Folder(a,Rc::new(f),PhantomData,PhantomData)}}
impl<I,O:Clone,F:Fn(O,&I)->O,D:CompFold<I,O,F>>
Computor<Duration,D> for Folder<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.comp_fold(self.0.clone(),self.1.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
