#![allow(unused_imports)]

use abra::adjustments::prelude::*;
use abra::prelude::*;
use abra::transform::prelude::*;

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/rotate.png";

pub fn main() {
  let mut image = Image::new_from_path(FILE);

  let start_time = std::time::Instant::now();

  rotate(&mut image, 45., None);
  color::threshold(&mut image, 128);

  println!("Rotation took: {:?}", start_time.elapsed());

  image.save(OUT_FILE, None);
}
