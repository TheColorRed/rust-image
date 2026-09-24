use abra::adjustments::prelude::*;
use abra::filters::prelude::blur::blur;
use abra::prelude::*;

const FILE: &str = "assets/kelsey.jpg";

fn main() {
  // Load image
  let mut image = Image::read(FILE).expect("Failed to read image");

  blur().apply(&mut image);

  image.write("out/blur.png", None).expect("Failed to write image");
}
