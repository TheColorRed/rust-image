#![allow(unused_imports)]
use abra::adjustments::prelude::levels::brightness;
use abra::adjustments::prelude::*;
use abra::options::prelude::{ApplyOptions, Hardware};
use abra::{filters, prelude::*};

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/adjustments.png";

pub fn main() {
  let mut image = Image::read(FILE).expect("Failed to load image");

  // let start_time = std::time::Instant::now();

  let area = Area::rect((100, 100), (200, 200)).with_feather(50);
  let apply = ApplyOptions::new().with_hardware(Hardware::Cpu); //.with_area(area);
  brightness(100) /* .with_options(apply) */
    .with_options(apply)
    .apply(&mut image);

  // color::threshold(128).apply(&mut image);
  // println!("Adjustment took: {:?}", start_time.elapsed());

  image.write(OUT_FILE, None).expect("Failed to save image");
}
