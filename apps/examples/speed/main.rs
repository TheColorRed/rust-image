use rand::Rng;
use std::io::{Write, stdin, stdout};
use std::time::Instant;

fn main() {
  let mut s = String::new();
  print!("Enter a value for u: ");
  let _ = stdout().flush();
  stdin().read_line(&mut s).expect("Did not enter a correct string.");
  let u: i32 = s.trim().parse().expect("Please enter a valid number.");

  let start_time = Instant::now();
  let r = rand::thread_rng().gen_range(0..10000);
  let mut a = vec![0; 10000];

  for i in 0..10000 {
    for _ in 0..100000 {
      a[i] += u;
    }
    a[i] += r as i32;
  }

  println!("{}", a[r]);
  println!("Time elapsed: {:?}", start_time.elapsed());
}
