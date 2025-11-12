use abra::{
  adjustments::color,
  color::{Color, Gradient},
  image::Image,
};

const FILE: &str = "assets/skirt.png";

pub fn main() {
  let mut img = Image::new_from_path(FILE);

  color::threshold(&mut img, 128);
  color::gradient_map(&mut img, Gradient::to_white(Color::blue()));

  img.save("out/mask.png", None);
}
