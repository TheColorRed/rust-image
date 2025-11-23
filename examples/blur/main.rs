use abra::{
  Area, Heart, Image,
  filters::{
    blur::{self, FocusBlurOptions},
    options::FilterOptions,
  },
};
use std::time::Instant;

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/blur.png";

fn main() {
  let mut image = Image::new_from_path(FILE);

  let start_time = Instant::now();
  // blur::focus_blur(&mut image, FocusBlurOptions::new());
  let area = Heart::new().fit((500, 500)).with_feather(5);
  // let area = Area::rect((100, 100), (200, 200));
  let options = FilterOptions::new().with_area(area.clone());
  blur::gaussian_blur(&mut image, 25, options);
  // blur::gaussian_blur(&mut image, 25, Area::rect((100, 100), (200, 200)).with_feather(30));
  image.save("out/focus-blur.png", None);

  // let start_time = Instant::now();
  // blur::box_blur(&mut image, 32);
  // println!("Box Blur took: {:?}", start_time.elapsed());
  // image.save(OUT_FILE, None);

  // // Motion blur example
  // let mut image_motion = Image::new_from_path(FILE);
  // let motion_start = Instant::now();
  // blur::motion_blur(&mut image_motion, 45.0, 20);
  // println!("Motion Blur took: {:?}", motion_start.elapsed());
  // image_motion.save("out/motion_blur.png", None);

  // // Surface blur example
  // let mut image_surface = Image::new_from_path(FILE);
  // let surface_start = Instant::now();
  // // Example: radius 16, threshold 25
  // blur::surface_blur(&mut image_surface, 16, 25);
  // println!("Surface Blur took: {:?}", surface_start.elapsed());
  // image_surface.save("out/surface_blur.png", None);
}
