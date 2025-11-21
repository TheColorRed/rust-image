use abra::{AspectRatio, Color, Heart, Image, Star, mask::Mask};

const FILE: &str = "assets/skirt.png";

pub fn main() {
  let image = Image::new_from_path(FILE);
  let mut mask = Mask::new_from_image(&image);

  let size = image.size();
  let star = Star::new().fit_with_aspect(size / 2, AspectRatio::meet());
  let heart = Heart::new().fit_with_aspect(size - 100, AspectRatio::meet());
  mask.draw_area(&star, Color::black(), None);
  mask.draw_area(&heart, Color::black(), (5, 200));

  mask.image().save("out/mask.png", None);
}
