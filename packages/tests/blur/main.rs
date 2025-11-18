#![allow(unused_imports)]
use abra::{filters::blur, image::Image};
use std::time::Instant;

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/blur.png";

fn main() {
  let mut image = Image::new_from_path(FILE);

  let start_time = Instant::now();
  blur::box_blur(&mut image, 32);
  println!("Box Blur took: {:?}", start_time.elapsed());
  image.save(OUT_FILE, None);

  // Motion blur example
  let mut image_motion = Image::new_from_path(FILE);
  let motion_start = Instant::now();
  blur::motion_blur(&mut image_motion, 45.0, 20);
  println!("Motion Blur took: {:?}", motion_start.elapsed());
  image_motion.save("out/motion_blur.png", None);

  // Surface blur example
  let mut image_surface = Image::new_from_path(FILE);
  let surface_start = Instant::now();
  // Example: radius 16, threshold 25
  blur::surface_blur(&mut image_surface, 16, 25);
  println!("Surface Blur took: {:?}", surface_start.elapsed());
  image_surface.save("out/surface_blur.png", None);
}
