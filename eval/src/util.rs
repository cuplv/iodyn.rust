/// takes two cumulative plots of change times in ms and prints out stats
pub fn print_crossover_stats(native_changes: &Vec<f64>, adapton_changes: &Vec<f64>) {
  let adapton_init = adapton_changes[0];
  let native_init = native_changes[0];
  let changes = native_changes.len() - 1;

  let update_time = adapton_changes.iter().zip(adapton_changes.iter().skip(1)).map(|(o,n)|{n-o}).sum::<f64>() / (adapton_changes.len() - 1) as f64;
  let crossover = native_changes.iter().skip(1).zip(adapton_changes.iter().skip(1)).enumerate()
    .fold((native_init,adapton_init,0),|(n,a,cross),(c,(&nt,&at))|{
      let new_cross = if n < a && nt >= at { c + 1 } else { cross };
      (nt,at,new_cross)
    }).2;

  println!(" - Native initial run: {:.2} ms",native_init);
  println!(" - Adapton initial run: {:.2} ms",adapton_init);
  println!(" - Adapton overhead: {:.2} (Adapton initial time / Native initial time)",adapton_init/native_init);
  println!(" - Adapton update time: {:.2} ms avg over the first {} changes",update_time,changes);
  if crossover > 0 {
    println!(" - Adapton cross over: {} changes  (When Adapton's update time overcomes its overhead)",crossover);
  }  else {
    println!(" - Adapton cross over off chart  (When Adapton's update time overcomes its overhead)");
  }
  println!(" - Adapton speedup: {:.2} (Native initial time / Adapton update time)",native_init/update_time);

}