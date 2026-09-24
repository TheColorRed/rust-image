use abra::canvas::prelude::*;
use abra::prelude::*;

const FILE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/layer-effects.png";

pub fn main() {
  let (width, height) = (512 + 100, 1024 + 100);
  let white_image = Image::new_from_color(width, height, Color::white());
  // For this simple example, save the solid white image directly.
  white_image.write(OUT_FILE, None).expect("Failed to save image");
}
