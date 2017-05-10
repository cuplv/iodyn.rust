use std::marker::PhantomData;
use std::rc::Rc;
use rand::{Rng,StdRng};
use time::Duration;
use primitives::*;
use adapton::engine::{Name,ns,name_fork};

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
	pub datagauge: usize,
	pub namegauge: usize,
	pub coord: G,
}
impl<D:CreateEmpty<G>,G:Rng>
Creator<Duration,D> for IncrementalEmpty<G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::inc_empty(self.datagauge, self.namegauge, &self.coord, rng)
	}
}

#[derive(Clone)]
pub struct IncrementalFrom<T,G:Rng> {
	pub data: T,
	pub datagauge: usize,
	pub namegauge: usize,
	pub coord: G,
}
impl<D:CreateFrom<T,G>,T:Clone,G:Rng>
Creator<Duration,D> for IncrementalFrom<T,G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::inc_from(self.data.clone(), self.datagauge, self.namegauge, &self.coord, rng)
	}
}

// builds a sequence from scratch, 
#[derive(Clone)]
pub struct IncrementalInit<G:Rng> {
	pub size: usize,
	pub datagauge: usize,
	pub namegauge: usize,
	pub coord: G,
}
impl<D:CreateInc<G>,G:Rng>
Creator<Duration,D> for IncrementalInit<G> {
	fn create(&mut self, rng: &mut StdRng) -> (Duration,D){
		D::inc_init(self.size, self.datagauge, self.namegauge, &self.coord, rng)
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

pub struct Native<I,O,F:Fn(&I)->O>(Rc<F>,PhantomData<I>,PhantomData<O>);
impl<I,O,F:Fn(&I)->O> Native<I,O,F> { pub fn new(f:F)->Self{Native(Rc::new(f),PhantomData,PhantomData)}}
impl<O,F,D>
Computor<Duration,D>
for Native<D::Input,O,F> where
	D:CompNative<O>,
	F:Fn(&D::Input)->O,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.comp_native(self.0.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
impl<O,F,D>
Computor<(Duration,O),D>
for Native<D::Input,O,F> where
	D:CompNative<O>,
	F:Fn(&D::Input)->O,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Duration,O) {
		data.comp_native(self.0.clone(),rng)
	}
}

pub struct Reverse;
impl<D: CompRev>
Computor<Duration,D> for Reverse {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = data.comp_rev(rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
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

pub struct TreeFoldNL<
	E,O,I:Fn(&E)->O,B:Fn(O,O)->O,M:Fn(O,u32,Option<Name>,O)->O
>{
	init: Rc<I>,
	bin: Rc<B>,
	binnl: Rc<M>,
	elm: PhantomData<E>,
	out: PhantomData<O>,
}
impl<E,O,I,B,M>
TreeFoldNL<E,O,I,B,M> where
	I:Fn(&E)->O,
	B:Fn(O,O)->O,
	M:Fn(O,u32,Option<Name>,O)->O,
{
	pub fn new(init:I,bin:B,binnl:M) -> Self {
		TreeFoldNL{
			init: Rc::new(init),
			bin: Rc::new(bin),
			binnl: Rc::new(binnl),
			elm: PhantomData,
			out: PhantomData,
		}
	}
}
impl<E,O,I,B,M,D>
Computor<Duration,D>
for TreeFoldNL<E,O,I,B,M> where
	I:Fn(&E)->O,
	B:Fn(O,O)->O,
	M:Fn(O,u32,Option<Name>,O)->O,
	D:CompTreeFoldNL<E,O,I,B,M>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time, answer) = data.comp_tfoldnl(self.init.clone(),self.bin.clone(),self.binnl.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}

pub struct TreeFoldG<
	E,O,I:Fn(&Vec<E>)->O,B:Fn(O,u32,Option<Name>,O)->O
>{
	init: Rc<I>,
	bin: Rc<B>,
	elm: PhantomData<E>,
	out: PhantomData<O>,
}
impl<E,O,I,B>
TreeFoldG<E,O,I,B> where
	I:Fn(&Vec<E>)->O,
	B:Fn(O,u32,Option<Name>,O)->O,
{
	pub fn new(init:I,bin:B) -> Self {
		TreeFoldG{
			init: Rc::new(init),
			bin: Rc::new(bin),
			elm: PhantomData,
			out: PhantomData,
		}
	}
}
impl<E,O,I,B,D>
Computor<Duration,D>
for TreeFoldG<E,O,I,B> where
	I:Fn(&Vec<E>)->O,
	B:Fn(O,u32,Option<Name>,O)->O,
	D:CompTreeFoldG<E,O,I,B>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time, answer) = data.comp_tfoldg(self.init.clone(),self.bin.clone(),rng);
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
impl<E,O,I,B,D>
Computor<(Duration,D::Target),D>
for TreeFoldG<E,O,I,B> where
	I:Fn(&Vec<E>)->O,
	B:Fn(O,u32,Option<Name>,O)->O,
	D:CompTreeFoldG<E,O,I,B>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Duration,D::Target) {
		data.comp_tfoldg(self.init.clone(),self.bin.clone(),rng)
	}
}

pub struct Mapper<I,O,F:Fn(&I)->O>{
	name: Name,
	mapfn: Rc<F>,
	elm: PhantomData<I>,
	out: PhantomData<O>,
}
impl<I,O,F:Fn(&I)->O> Mapper<I,O,F> {
	pub fn new(n:Name,f:F) -> Self {Mapper{
		name: n,
		mapfn: Rc::new(f),
		elm: PhantomData,
		out: PhantomData,
	}}
}
impl<I,O,F:Fn(&I)->O,D:CompMap<I,O,F>>
Computor<Duration,D> for Mapper<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,answer) = ns(self.name.clone(),||{
			data.comp_map(self.mapfn.clone(),rng)
		});
		#[allow(unused)]
		let saver = Vec::new().push(answer); // don't let rust compile this away
		time
	}
}
impl<I,O,F:Fn(&I)->O,D:CompMap<I,O,F>>
Computor<(Duration,D::Target),D> for Mapper<I,O,F> {
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Duration,D::Target) {
		ns(self.name.clone(),||{
			data.comp_map(self.mapfn.clone(),rng)
		})
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

pub struct MFolder<
	A:Clone, E, M, T, O, R:Fn(A,&E)->A, N:Fn(A,M)->A, F:Fn(T)->O
>{
	name: Name,
	init:A,
	run: Rc<R>,
	run_meta: Rc<N>,
	finish: Rc<F>,
	accum: PhantomData<A>,
	elm: PhantomData<E>,
	meta: PhantomData<M>,
	target: PhantomData<T>,
	out: PhantomData<O>,
}

impl<
	A:Clone, E, M, T, O, R:Fn(A,&E)->A, N:Fn(A,M)->A, F:Fn(T)->O
> MFolder<A,E,M,T,O,R,N,F> {
	pub fn new(name:Name,a:A,r:R,m:N,f:F) -> Self {
		MFolder{
			name: name,
			init: a,
			run: Rc::new(r),
			run_meta: Rc::new(m),
			finish: Rc::new(f),
			accum: PhantomData,
			elm: PhantomData,
			meta: PhantomData,
			target: PhantomData,
			out: PhantomData,
		}
	}
}
impl<A,E,M,O,R,N,F,D>
Computor<Duration,D>
for MFolder<A,E,M,D::Target,O,R,N,F> 
where
	A:Clone,
	R:Fn(A,&E)->A,
	N:Fn(A,M)->A,
	F:Fn(D::Target)->O,
	D:CompFoldMeta<E,A,M,R,N>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,run) = ns(self.name.clone(), ||{
			data.comp_fold_meta(self.init.clone(), self.run.clone(), self.run_meta.clone(), rng)
		});
		let finish = ns(name_fork(self.name.clone()).0,||{(
			self.finish)(run)
		});
		#[allow(unused)]
		let saver = Vec::new().push(finish); // don't let rust compile this away
		time
	}
}
impl<A,E,M,O,R,N,F,D>
Computor<(Vec<Duration>,O),D>
for MFolder<A,E,M,D::Target,O,R,N,F> 
where
	A:Clone,
	R:Fn(A,&E)->A,
	N:Fn(A,M)->A,
	F:Fn(D::Target)->O,
	D:CompFoldMeta<E,A,M,R,N>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Vec<Duration>,O) {
		let (time,run) = ns(self.name.clone(), ||{
			data.comp_fold_meta(self.init.clone(), self.run.clone(), self.run_meta.clone(), rng)
		});
		let finish = ns(name_fork(self.name.clone()).0,||{(
			self.finish)(run)
		});
		let mut v = Vec::new();
		v.push(time);
		(v,finish)
	}
}

pub struct HFolder<
	A:Clone, E, M, T, O, R:Fn(A,&E)->A, RF:Fn(A,Option<Name>)->A, N:Fn(A,M)->A, F:Fn(T)->O
>{
	name: Name,
	init:A,
	run: Rc<R>,
	run_fin: Rc<RF>,
	run_meta: Rc<N>,
	finish: Rc<F>,
	accum: PhantomData<A>,
	elm: PhantomData<E>,
	meta: PhantomData<M>,
	target: PhantomData<T>,
	out: PhantomData<O>,
}

impl<
	A:Clone, E, M, T, O, R:Fn(A,&E)->A, RF:Fn(A,Option<Name>)->A, N:Fn(A,M)->A, F:Fn(T)->O
> HFolder<A,E,M,T,O,R,RF,N,F> {
	pub fn new(name:Name,a:A,r:R,rf:RF,m:N,f:F) -> Self {
		HFolder{
			name: name,
			init: a,
			run: Rc::new(r),
			run_fin: Rc::new(rf),
			run_meta: Rc::new(m),
			finish: Rc::new(f),
			accum: PhantomData,
			elm: PhantomData,
			meta: PhantomData,
			target: PhantomData,
			out: PhantomData,
		}
	}
}
impl<A,E,M,O,R,RF,N,F,D>
Computor<Duration,D>
for HFolder<A,E,M,D::Target,O,R,RF,N,F> 
where
	A:Clone,
	R:Fn(A,&E)->A,
	RF:Fn(A,Option<Name>)->A,
	N:Fn(A,M)->A,
	F:Fn(D::Target)->O,
	D:CompFoldArchive<E,A,M,R,RF,N>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> Duration {
		let (time,run) = ns(self.name.clone(), ||{
			data.comp_fold_archive(self.init.clone(), self.run.clone(), self.run_fin.clone(), self.run_meta.clone(), rng)
		});
		let finish = ns(name_fork(self.name.clone()).0,||{(
			self.finish)(run)
		});
		#[allow(unused)]
		let saver = Vec::new().push(finish); // don't let rust compile this away
		time
	}
}
impl<A,E,M,O,R,RF,N,F,D>
Computor<(Vec<Duration>,O),D>
for HFolder<A,E,M,D::Target,O,R,RF,N,F> 
where
	A:Clone,
	R:Fn(A,&E)->A,
	RF:Fn(A,Option<Name>)->A,
	N:Fn(A,M)->A,
	F:Fn(D::Target)->O,
	D:CompFoldArchive<E,A,M,R,RF,N>,
{
	fn compute(&mut self, data: &D, rng: &mut StdRng) -> (Vec<Duration>,O) {
		let (time,run) = ns(self.name.clone(), ||{
			data.comp_fold_archive(self.init.clone(), self.run.clone(), self.run_fin.clone(), self.run_meta.clone(), rng)
		});
		let finish = ns(name_fork(self.name.clone()).0,||{(
			self.finish)(run)
		});
		let mut v = Vec::new();
		v.push(time);
		(v,finish)
	}
}

