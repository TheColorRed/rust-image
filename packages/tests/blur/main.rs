#![allow(unused_imports)]
use abra::{filters::blur, image::Image};

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/blur.png";

fn main() {
  let mut image = Image::new_from_path(FILE);

  let start_time = std::time::Instant::now();
  blur::box_blur(&mut image, 32);
  println!("Blur took: {:?}", start_time.elapsed());

  image.save(OUT_FILE, None);
}
