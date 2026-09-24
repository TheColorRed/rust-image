#![allow(unused_imports)]

use abra::adjustments::prelude::*;
use abra::prelude::*;
use abra::transform::prelude::*;

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/rotate.png";

pub fn main() {
  let mut image = Image::read(FILE).expect("Failed to load image");

  let start_time = std::time::Instant::now();

  image.rotate(45., None);
  color::threshold(128).apply(&mut image);

  println!("Rotation took: {:?}", start_time.elapsed());

  image.write(OUT_FILE, None).expect("Failed to save image");
}
