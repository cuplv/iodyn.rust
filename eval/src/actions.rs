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

#[derive(Clone)]
pub struct IncrementalEmpty<G:Rng> {
	pub unitgauge: usize,
	pub namegauge: usize,
	pub coord: G,
}
impl<D:CreateEmpty<G>,G:Rng>
Creator<Duration,D> for IncrementalEmpty<G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::inc_empty(self.unitgauge, self.namegauge, &self.coord, rng)
	}
}

#[derive(Clone)]
pub struct IncrementalFrom<T,G:Rng> {
	pub data: T,
	pub unitgauge: usize,
	pub namegauge: usize,
	pub coord: G,
}
impl<D:CreateFrom<T,G>,T:Clone,G:Rng>
Creator<Duration,D> for IncrementalFrom<T,G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::inc_from(self.data.clone(), self.unitgauge, self.namegauge, &self.coord, rng)
	}
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

/// Insert multiple elements into the collection at random places
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

pub struct Compute2<A,B,I,J,K>(A,B,PhantomData<I>,PhantomData<J>,PhantomData<K>) where
	A: Computor<(Vec<Duration>,J),I>,
	B: Computor<(Vec<Duration>,K),J>
;
impl<A,B,I,J,K> Compute2<A,B,I,J,K> where
	A: Computor<(Vec<Duration>,J),I>,
	B: Computor<(Vec<Duration>,K),J>
{
	pub fn new(a:A,b:B)->Self{Compute2(a,b,PhantomData,PhantomData,PhantomData)}
}
impl<A,B,I,J,K>
Computor<(Vec<Duration>,K),I>
for Compute2<A,B,I,J,K> where
	A: Computor<(Vec<Duration>,J),I>,
	B: Computor<(Vec<Duration>,K),J>
{
	fn compute(&mut self, data: &I, rng: &mut StdRng) -> (Vec<Duration>,K) {
		let mut times = Vec::new();
		let (duration,data) = (self.0).compute(data,rng);
		times.extend(duration);
		let (duration,data) = (self.1).compute(&data,rng);
		times.extend(duration);
		(times,data)
	}
}

pub struct Proj0;
impl<R:Clone,E> Computor<(Vec<Duration>,R),(R,E)> for Proj0 {
	fn compute(&mut self, data: &(R,E), _rng: &mut StdRng) -> (Vec<Duration>,R) {
		(Vec::new(),data.0.clone())
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
Computor<Duration,D>
for Folder<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.comp_fold(self.0.clone(),self.1.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
impl<I,O:Clone,F:Fn(O,&I)->O,D:CompFold<I,O,F>>
Computor<(Duration,D::Target),D>
for Folder<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Duration,D::Target) {
		data.comp_fold(self.0.clone(),self.1.clone(),rng)
	}
}
impl<I,O:Clone,F:Fn(O,&I)->O,D:CompFold<I,O,F>>
Computor<(Vec<Duration>,D::Target),D>
for Folder<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Vec<Duration>,D::Target) {
		let (time,answer) = data.comp_fold(self.0.clone(),self.1.clone(),rng);
		let mut v = Vec::new();
		v.push(time);
		(v,answer)
	}
}

pub struct FFolder<
	A:Clone, E, T, O, R:Fn(A,&E)->A, F:Fn(T)->O
>{
	init:A,
	run: Rc<R>,
	finish: Rc<F>,
	accum: PhantomData<A>,
	elm: PhantomData<E>,
	target: PhantomData<T>,
	out: PhantomData<O>,
}

impl<A:Clone, E, T, O, R:Fn(A,&E)->A, F:Fn(T)->O> FFolder<A,E,T,O,R,F> {
	pub fn new(a:A,r:R,f:F) -> Self {
		FFolder{
			init: a,
			run: Rc::new(r),
			finish: Rc::new(f),
			accum: PhantomData,
			elm: PhantomData,
			target: PhantomData,
			out: PhantomData,
		}
	}
}
impl<A:Clone, E, O, R:Fn(A,&E)->A, F:Fn(D::Target)->O, D:CompFold<E,A,R>>
Computor<Duration,D>
for FFolder<A,E,D::Target,O,R,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,run) = data.comp_fold(self.init.clone(),self.run.clone(),rng);
		let finish = (self.finish)(run);
		#[allow(unused)]
		let saver = Vec::new().push(finish); // don't let rust compile this away
		time
	}
}
impl<A:Clone, E, O, R:Fn(A,&E)->A, F:Fn(D::Target)->O, D:CompFold<E,A,R>>
Computor<(Vec<Duration>,O),D>
for FFolder<A,E,D::Target,O,R,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Vec<Duration>,O) {
		let (time,run) = data.comp_fold(self.init.clone(),self.run.clone(),rng);
		let finish = (self.finish)(run);
		let mut v = Vec::new();
		v.push(time);
		(v,finish)
	}
}

