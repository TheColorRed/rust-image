use abra::prelude::*;
use abra::tools::prelude::*;

// const FILE: &str = "assets/nude/ellen.jpg";
const FILE: &str = "assets/nude/black-hair-chest-brown-hair-long-hair.jpg";

fn main() {
  // Load image
  let mut image = Image::read(FILE).expect("Failed to read image");

  // eyes
  // let start = PointF::new(350, 300);
  // let end = PointF::new(450, 250);

  // nipples
  let start = PointF::new(100, 950);
  let end = PointF::new(600, 1050);
  // let start = PointF::new(707, 1352);
  // let end = PointF::new(707 + 170, 1352 - 18);

  straighten(start, end).apply(&mut image);

  image.write("out/straighten.png", None).expect("Failed to write image");
}
