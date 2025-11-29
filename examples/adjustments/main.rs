#![allow(unused_imports)]
use abra::adjustments::prelude::*;
use abra::{filters, prelude::*};

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/adjustments.png";

pub fn main() {
  let mut image = Image::new_from_path(FILE);

  let start_time = std::time::Instant::now();
  color::threshold(&mut image, 128);
  println!("Adjustment took: {:?}", start_time.elapsed());

  image.save(OUT_FILE, None);
}
