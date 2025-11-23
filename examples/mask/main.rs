use abra::{AspectRatio, Color, Heart, Image, Star, mask::Mask};

const FILE: &str = "assets/skirt.png";

pub fn main() {
  let image = Image::new_from_path(FILE);
  let mut mask = Mask::new_from_image(&image);

  let size = image.size();
  let star = Star::new().fit_with_aspect(size / 2, AspectRatio::meet());
  let heart = Heart::new().fit_with_aspect(size - 100, AspectRatio::meet());
  mask.draw_area(&star.with_feather(30), Color::black(), None);
  mask.draw_area(&heart.with_feather(30), Color::black(), (5, 200));

  // Save the mask image for debugging
  mask.image().save("out/mask.png", None);

  // Apply the mask to the image and save the result
  let mut masked = image;
  mask.apply_to_image(&mut masked);
  masked.save("out/masked.png", None);
}
