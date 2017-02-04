use std::rc::Rc;
use rand::StdRng;
use adapton::engine::*;
use pmfp_collections::{IRaz, IRazTree};
use pmfp_collections::inc_tree_cursor::gen_level;
use Params;
use eval::*;

pub struct EvalIRaz<E:Eval,G:ElemGen<E>> {
	// Option for cleaner code, None means uninitialized
	raztree: Option<IRazTree<E>>,
	names: usize,
	data: G,
}

impl<E: Eval,G:ElemGen<E>> EvalIRaz<E,G> {
	pub fn new(data:G) -> Self {
		EvalIRaz {
			raztree: None,
			names: 0,
			data: data,
		}
	}
	pub fn next_name(&mut self) -> Name {
		let n = self.names;
		self.names += 1;
		name_of_usize(n)
	}
}

impl<E:Eval,G:ElemGen<E>> DataInit<E,G> for EvalIRaz<E,G> {
	fn init(p: &Params, data: G, mut rng: &mut StdRng) -> Self
	{
		let mut eval = EvalIRaz::new(data);
		let mut raz = IRaz::new();
		let names = p.start/(p.namesize*p.unitsize);
		for _ in 0..names {
			for n in 0..p.namesize {
				for _ in 0..p.unitsize {
					raz.push_left(eval.data.gen(p));
				}
				if n < p.namesize - 1 {
					raz.archive_left(gen_level(rng), None);
				}
			}
			raz.archive_left(gen_level(rng), Some(eval.next_name()));
		}
		let more = p.start % (p.namesize*p.unitsize);
		let units = more / p.unitsize;
		for _ in 0..units {
			for _ in 0..p.unitsize {
				raz.push_left(eval.data.gen(p));
			}
			raz.archive_left(gen_level(rng), None);
		}
		let moremore = more % p.unitsize;
		for _ in 0..moremore {
			raz.push_left(eval.data.gen(p));
		}
		eval.raztree = Some(raz.unfocus());
		eval
	}
}

impl<E:Eval,G:ElemGen<E>> DataAppend for EvalIRaz<E,G> {
	fn append(mut self, p: &Params, rng: &mut StdRng) -> Self {
		let tree = self.raztree.take().unwrap();
		let mut len = tree.len();
		let mut zip = tree.focus(len).unwrap();
		zip.push_left(self.data.gen(p));
		len +=1;
		if len % p.unitsize == 0 {
			let name = if len % p.namesize == 0 {
				Some(self.next_name())
			} else { None };
			zip.archive_left(gen_level(rng),name);
		}
		self.raztree = Some(zip.unfocus());
		self
	}
}

impl<E:Eval+Ord,G:ElemGen<E>> DataMax<E> for EvalIRaz<E,G> {
	fn max(&self, _rng: &mut StdRng) -> Option<E> {
    self.raztree.clone().unwrap().fold_up(Rc::new(|e:&E|e.clone()),Rc::new(|e1:E,e2:E|::std::cmp::max(e1,e2)))
	}
}

