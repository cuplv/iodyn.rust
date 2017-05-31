extern crate rand;
#[macro_use] extern crate clap;
#[macro_use] extern crate adapton;
extern crate adapton_lab;
extern crate iodyn;
extern crate eval;

use std::rc::Rc;
use std::collections::LinkedList;
use std::io::{BufWriter,Write};
use std::fs::File;
use rand::{StdRng,SeedableRng};
use adapton::reflect;
use adapton::engine::*;
use adapton::macros::*;
use adapton::engine::manage::*;
use adapton_lab::labviz::*;
#[allow(unused)] use iodyn::{IRaz,IRazTree};
#[allow(unused)] use iodyn::archive_stack::{AtTail,AStack as IAStack};
#[allow(unused)] use eval::eval_iraz::EvalIRaz;
#[allow(unused)] use eval::eval_vec::EvalVec;
#[allow(unused)] use eval::eval_llist::EvalLList;
#[allow(unused)] use eval::eval_iastack::EvalIAStack;
#[allow(unused)] use eval::accum_lists::*;
#[allow(unused)] use eval::types::*;
use eval::actions::*;
use eval::test_seq::{TestResult,EditComputeSequence,aggregate};
use eval::util::*;

use geom::*;
mod geom {
  #[derive(Clone, Debug, Hash, Eq, PartialEq)]
  pub struct Point {
    pub x: isize,
    pub y: isize,
  }
  use rand::{Rng,Rand};
  impl Rand for Point{
    fn rand<R: Rng>(rng: &mut R) -> Self {
      // max must be less than 3_000_000_000 to avoid overflow on 64bit machine
      let max = 1_000_000_000;
      Point{x:rng.gen::<isize>() % max,y: rng.gen::<isize>() % max}
    }
  }

  #[derive(Clone, Debug, Hash, Eq, PartialEq)]
  pub struct Line {
    pub u: Point,
    pub v: Point,
  }


  // Point operation functions
  pub fn point_subtract<'a>(u: &'a Point, v: &'a Point) -> Point {
    // Finds the difference between u and v
    Point { x: u.x - v.x, y: u.y - v.y}
  }

  pub fn magnitude(pt: &Point) -> f32 {
    // Finds the magnitude of position vector for pt
    (((pt.x * pt.x) + (pt.y * pt.y)) as f32).sqrt()
  }

  pub fn cross_prod(u: &Point, v: &Point) -> isize {
    // The corss product of points u and v
    (u.x * v.y) - (u.y * v.x)
  }

  pub fn line_point_dist(l: &Line, p: &Point) -> f32 {
    let d1 = point_subtract(&l.v, &l.u);
    let d2 = point_subtract(&l.u, &p);
    let d3 = point_subtract(&l.v, &l.u);  
    ((cross_prod(&d1, &d2) as f32) / magnitude(&d3)).abs()
  }

  pub fn line_side_test(l: &Line, p: &Point) -> bool {
    // Tests which side of the line a point is on
    if (l.u == *p) || (l.v == *p) {
      false
    } else {
      let d1 = point_subtract(&l.v, &l.u);
      let d2 = point_subtract(&l.u, &p);
      let c = cross_prod(&d1, &d2);
      if c <= 0 {
        false
      } else {
        true
      }
    }
  }
}

fn main () {

// provide additional stack memory
  let child =
    std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(move || { 
      main2()
    });
  let _ = child.unwrap().join();
}
fn main2() {
// end provide additional stack memory

  //command-line
  let args = clap::App::new("quickhull")
    .version("0.1")
    .author("Kyle Headley <kyle.headley@colorado.edu>")
    .args_from_usage("\
      --dataseed=[dataseed]			  'seed for random data'
      --editseed=[editseed]      'seed for random edits (and misc.)'
      -s, --start=[start]         'starting sequence length'
      -g, --datagauge=[datagauge] 'initial elements per structure unit'
      -n, --namegauge=[namegauge] 'initial tree nodes between each art'
      -e, --edits=[edits]         'edits per batch'
      -c, --changes=[changes]     'number of incremental changes'
      -t, --trials=[trials]       'number of runs to aggregate stats from'
      -o, --outfile=[outfile]     'name for output files (of different extensions)'
      --trace                     'produce an output trace of the incremental run' ")
    .get_matches();
  let dataseed = value_t!(args, "dataseed", usize).unwrap_or(0);
  let editseed = value_t!(args, "editseed", usize).unwrap_or(0);
	let start_size = value_t!(args, "start", usize).unwrap_or(1_000_000);
	let datagauge = value_t!(args, "datagauge", usize).unwrap_or(1_000);
	let namegauge = value_t!(args, "namegauge", usize).unwrap_or(1);
	let edits = value_t!(args, "edits", usize).unwrap_or(1);
  let changes = value_t!(args, "changes", usize).unwrap_or(30);
  let trials = value_t!(args, "trials", usize).unwrap_or(1);
  let outfile = args.value_of("outfile");
  let do_trace = args.is_present("trace");
  let coord = StdRng::from_seed(&[dataseed]);

  let mut test_raz = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: Native::new(|ps:&IRazTree<Point>|{
      let most_left = ns(name_of_str("left_most"),||ps.clone().fold_up(
        Rc::new(|a:&Point|a.clone()),
        Rc::new(|p:Point,q:Point| { if p.x < q.x { p } else { q }}),
      )).unwrap();
      let most_right = ns(name_of_str("right_most"),||ps.clone().fold_up(
        Rc::new(|a:&Point|a.clone()),
        Rc::new(|p:Point,q:Point| { if p.x > q.x { p } else { q }}),
      )).unwrap();
      let t_line = Line { u: most_left.clone(), v: most_right.clone() };
      let b_line = Line { u: most_right.clone(), v: most_left.clone() };
      let t_points = ns(name_of_str("top_points"),||above_line(&ps,t_line.clone()));
      let b_points = ns(name_of_str("bottom_points"),||above_line(&ps,b_line.clone()));
      let mut hull = IRaz::new();
      hull.push_left(most_left);
      let (nb,nt) = name_fork(name_unit());
      let mut hull = ns(name_of_str("bottom_rec"),||memo!(name_unit() =>> quickhull_rec, n:nb, l:b_line, p:b_points, h:hull));
      hull.push_left(most_right);
      ns(name_of_str("hull"),||{hull.archive_left(iodyn::inc_level(),Some(name_unit()))});
      let hull = ns(name_of_str("top_rec"),||memo!(name_unit() =>> quickhull_rec, n:nt, l:t_line, p:t_points, h:hull));
      //println!("unfocus and return");
      //return ns(name_of_str("unfocus"),||{hull.memo_unfocus()});
      return hull.unfocus();
      fn above_line(ps: &IRazTree<Point>, line: Line) -> IRazTree<Point> {
        // build a filtered version of the input tree
        ps.clone().fold_up_gauged(Rc::new(move|v:&Vec<Point>|{
          let mut c = v.clone();
          c.retain(|p|{line_side_test(&line,p)});
          match IRazTree::from_vec(c) {
            None => IRazTree::empty(),
            Some(t) => t,
          }
        }),Rc::new(|t1:IRazTree<Point>,lev,nm,t2:IRazTree<Point>|{
          match (t1.is_empty(),t2.is_empty()) {
            (true,true) => IRazTree::empty(),
            (false,true) => t1,
            (true,false) => t2,
            (false,false) => {IRazTree::join(t1,lev,nm,t2).unwrap()},
          }
        })).unwrap()
      }
      fn quickhull_rec(nm: Name, line:Line, ps:IRazTree<Point>, hull:IRaz<Point>) -> IRaz<Point>{
        //println!("rec_line: {:?}", line);
        if ps.is_empty() { return hull; }
        let l = line.clone();
        let mid = ns(name_of_str("mid"),||ps.clone().fold_up(Rc::new(|a:&Point|a.clone()),Rc::new(move|p,q|{
          let dp = line_point_dist(&l, &p);
          let dq = line_point_dist(&l, &q);
          if dp > dq { p } else { q }
        }))).unwrap();
        let l_line = Line{u: line.u.clone(), v: mid.clone() };
        let r_line = Line{u: mid.clone(), v: line.v.clone() };
        let l_points = ns(name_of_str("left_points"),||above_line(&ps,l_line.clone()));
        let r_points = ns(name_of_str("right_points"),||above_line(&ps,r_line.clone()));
        let (nl,nr) = name_fork(nm.clone());
        let mut hull = ns(name_of_str("left_rec"),||memo!(name_unit() =>> quickhull_rec, n:nl, l:l_line, p:l_points, h:hull));
        hull.push_left(mid);
        ns(name_of_str("hull"),||{hull.archive_left(iodyn::inc_level(),Some(nm))});
        let hull = ns(name_of_str("right_rec"),||memo!(name_unit() =>> quickhull_rec, n:nr, l:r_line, p:r_points, h:hull));
        hull
      }
    }),
    changes: changes,
  };
  let mut test_vec = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: Native::new(|ps:&Vec<Point>|{
      let most_left = ps.iter().fold(
        Point{x:100,y:0},
        |p,q| { if p.x < q.x { p } else { q.clone() }},
      );
      let most_right = ps.iter().fold(
        Point{x:-100,y:0},
        |p,q| { if p.x > q.x { p } else { q.clone() }},
      );
      let t_line = Line { u: most_left.clone(), v: most_right.clone() };
      let b_line = Line { u: most_right.clone(), v: most_left.clone() };
      let mut t_points = ps.clone();
      t_points.retain(|p|line_side_test(&t_line,p));
      let mut b_points = ps.clone();
      b_points.retain(|p|line_side_test(&b_line,p));
      let mut hull = Vec::new();
      hull.push(most_left);
      let mut hull = quickhull_rec(b_line, b_points, hull);
      hull.push(most_right);
      let hull = quickhull_rec(t_line, t_points, hull);
      return hull;
      fn quickhull_rec(l:Line, ps:Vec<Point>, hull:Vec<Point>) -> Vec<Point>{
        if ps.is_empty() { return hull; }
        let mid = ps.iter().fold(l.u.clone(),|p,q|{
          let dp = line_point_dist(&l, &p);
          let dq = line_point_dist(&l, &q);
          if dp > dq { p } else { q.clone() }
        });
        let l_line = Line{u: l.u.clone(), v: mid.clone() };
        let r_line = Line{u: mid.clone(), v: l.v.clone() };
        let mut l_points = ps.clone();
        l_points.retain(|p|line_side_test(&l_line,p));
        let mut r_points = ps.clone();
        r_points.retain(|p|line_side_test(&r_line,p));
        let mut hull = quickhull_rec(l_line, l_points, hull);
        hull.push(mid);
        let hull = quickhull_rec(r_line, r_points, hull);
        hull
      }
    }),
    changes: changes,
  };
  let mut test_list = EditComputeSequence{
    init: IncrementalInit {
      size: start_size,
      datagauge: datagauge,
      namegauge: namegauge,
      coord: coord.clone(),
    },
    edit: BatchInsert(edits),
    comp: Native::new(|ps:&LinkedList<Point>|{
      let most_left = ps.iter().fold(
        Point{x:100,y:0},
        |p,q| { if p.x < q.x { p } else { q.clone() }},
      );
      let most_right = ps.iter().fold(
        Point{x:-100,y:0},
        |p,q| { if p.x > q.x { p } else { q.clone() }},
      );
      let t_line = Line { u: most_left.clone(), v: most_right.clone() };
      let b_line = Line { u: most_right.clone(), v: most_left.clone() };
      let t_points = ps.iter().filter(|p|line_side_test(&t_line,p)).map(|p|p.clone()).collect::<LinkedList<_>>();
      let b_points = ps.iter().filter(|p|line_side_test(&b_line,p)).map(|p|p.clone()).collect::<LinkedList<_>>();
      let mut hull = LinkedList::new();
      hull.push_back(most_left.clone());
      let mut hull = quickhull_rec(b_line, b_points, hull);
      hull.push_back(most_right.clone());
      let hull = quickhull_rec(t_line, t_points, hull);
      return hull;
      fn quickhull_rec(l:Line, ps:LinkedList<Point>, hull:LinkedList<Point>) -> LinkedList<Point>{
        if ps.is_empty() { return hull; }
        let mid = ps.iter().fold(l.u.clone(),|p,q|{
          let dp = line_point_dist(&l, &p);
          let dq = line_point_dist(&l, &q);
          if dp > dq { p } else { q.clone() }
        });
        let l_line = Line{u: l.u.clone(), v: mid.clone() };
        let r_line = Line{u: mid.clone(), v: l.v.clone() };
        let l_points = ps.iter().filter(|p|line_side_test(&l_line,p)).map(|p|p.clone()).collect::<LinkedList<_>>();
        let r_points = ps.iter().filter(|p|line_side_test(&r_line,p)).map(|p|p.clone()).collect::<LinkedList<_>>();
        let mut hull = quickhull_rec(l_line, l_points, hull);
        hull.push_back(mid.clone());
        let hull = quickhull_rec(r_line, r_points, hull);
        hull
      }
    }),
    changes: changes,
  };


  // run experiments

  init_dcg(); assert!(engine_is_dcg());

  let mut rng = StdRng::from_seed(&[editseed]);

  let mut results_non_inc: Vec<TestResult<
    EvalVec<Point,StdRng>,
    Vec<_>,
  >> = Vec::new();
  let mut results_list: Vec<TestResult<
    EvalLList<Point,StdRng>,
    LinkedList<_>,
  >> = Vec::new();
  let mut results_inc: Vec<TestResult<
    EvalIRaz<Point,StdRng>,
    IRazTree<_>,
  >> = Vec::new();

  for i in 0..trials+1 {
    //reseed rng
    let new_seed = &[dataseed+i];
    test_vec.init.coord.reseed(new_seed);
    test_list.init.coord.reseed(new_seed);
    test_raz.init.coord.reseed(new_seed);

    results_non_inc.push(test_vec.test(&mut rng));
    results_list.push(test_list.test(&mut rng));
    // for visual debugging
    if do_trace && i == 1 {reflect::dcg_reflect_begin()}

    ns(name_of_usize(i),||results_inc.push(test_raz.test(&mut rng)));

    if do_trace && i == 1 {
      let traces = reflect::dcg_reflect_end();

      // output trace
      let f = File::create("trace.html").unwrap();
      let mut writer = BufWriter::new(f);
      writeln!(writer, "{}", style_string()).unwrap();
      writeln!(writer, "<div class=\"label\">Editor trace({}):</div>", traces.len()).unwrap();
      writeln!(writer, "<div class=\"traces\">").unwrap();
      for tr in traces {
        div_of_trace(&tr).write_html(&mut writer);
      }
    }
    // correctness check

    let non_inc_comparison =
      results_non_inc[i].result_data
      .clone().into_iter()
      .collect::<Vec<_>>()
    ;
    let list_comparison =
      results_list[i].result_data
      .clone().into_iter()
      .collect::<Vec<_>>()
    ;
    let inc_comparison =
      results_inc[i].result_data
      .clone().into_iter()
      .collect::<Vec<_>>()
    ;
    if non_inc_comparison != inc_comparison || non_inc_comparison != list_comparison {
      println!("Final results({}) differ:",i);
      println!("the incremental results({}): {:?}", inc_comparison.len(),inc_comparison);
      println!("non incremental results({}): {:?}", non_inc_comparison.len(),non_inc_comparison);
      println!("the linked list results({}): {:?}", list_comparison.len(),list_comparison);
      println!("This is an error");
      ::std::process::exit(1);
    }
  }
  println!("Final results from all runs of all versions match.");

  let summary_inc = aggregate(&results_inc[1..]); // slice to remove warmup run
  let summary_non_inc = aggregate(&results_non_inc[1..]);
  let summary_list = aggregate(&results_list[1..]);

  println!("At input size: {}, Average of {} trials after warmup",start_size,trials);
  print_crossover_stats(&summary_non_inc.computes,&summary_inc.computes);

  // generate data file

  let filename = if let Some(f) = outfile {f} else {"out"};
  println!("Generating {}.pdf ...", filename);

  let mut dat = File::create(filename.to_owned()+".dat").unwrap();

  writeln!(dat,"#non inc").unwrap();
  summary_non_inc.write_to(&mut dat);
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  writeln!(dat,"#inc").unwrap();
  summary_inc.write_to(&mut dat);
  writeln!(dat,"").unwrap();
  writeln!(dat,"").unwrap();
  writeln!(dat,"#list").unwrap();
  summary_list.write_to(&mut dat);

  // generate plot script

  let mut plotscript = File::create(filename.to_owned()+".plotscript").unwrap();

  writeln!(plotscript,"set terminal pdf").unwrap();
  writeln!(plotscript,"set output '{}.pdf'", filename).unwrap();
  write!(plotscript,"set title \"{}", "Cumulative time to calculate quickhull after inserting element(s)\\n").unwrap();
  writeln!(plotscript,"(s)ize: {}, (g)auge: {}, (n)ame-gauge: {}, (e)dit-batch: {}, (t)rials: {}\"", start_size,datagauge,namegauge,edits,trials).unwrap();
  writeln!(plotscript,"set xlabel '{}'", "(c)hanges").unwrap();
  writeln!(plotscript,"set ylabel '{}'","Time(ms)").unwrap();
  writeln!(plotscript,"set key left top box").unwrap();
  writeln!(plotscript,"set grid ytics mytics  # draw lines for each ytics and mytics").unwrap();
  writeln!(plotscript,"set grid xtics mxtics  # draw lines for each xtics and mxtics").unwrap();
  writeln!(plotscript,"set mytics 5           # set the spacing for the mytics").unwrap();
  writeln!(plotscript,"set mxtics 5           # set the spacing for the mxtics").unwrap();
  writeln!(plotscript,"set grid               # enable the grid").unwrap();
  writeln!(plotscript,"plot \\").unwrap();
  writeln!(plotscript,"'{}.dat' i 0 u 1:6:7 ls 1 t '{}' with errorbars,\\",filename,"Non-incremental Compute Time").unwrap();
  writeln!(plotscript,"'{}.dat' i 0 u 1:6 ls 1 notitle with linespoints,\\",filename).unwrap();
  writeln!(plotscript,"'{}.dat' i 1 u 1:6:7 ls 2 t '{}' with errorbars,\\",filename,"Incremental Compute Time").unwrap();
  writeln!(plotscript,"'{}.dat' i 1 u 1:6 ls 2 notitle with linespoints,\\",filename).unwrap();
  writeln!(plotscript,"'{}.dat' i 2 u 1:6:7 ls 3 t '{}' with errorbars,\\",filename,"Linked List Compute Time").unwrap();
  writeln!(plotscript,"'{}.dat' i 2 u 1:6 ls 3 notitle with linespoints,\\",filename).unwrap();

  // generate plot

  ::std::process::Command::new("gnuplot").arg(filename.to_owned()+".plotscript").output().unwrap();

}
