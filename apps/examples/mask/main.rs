use abra::mask::prelude::*;
use abra::prelude::*;

const FILE: &str = "assets/skirt.png";

pub fn main() {
  let image = Image::read(FILE).expect("Failed to load image");
  let mut mask = Mask::new_from_image(&image);

  let size = image.size();
  let star = Area::shape(Shape::Star).fit(size / 2, AspectRatio::meet());
  let heart = Area::shape(Shape::Heart).fit(size - 100, AspectRatio::meet());
  mask.draw_area(&star.with_feather(30), Color::black(), None);
  mask.draw_area(&heart.with_feather(30), Color::black(), (5, 200));

  // Save the mask image for debugging
  mask.to_image().write("out/mask.png", None).expect("Failed to save mask");

  // Apply the mask to the image and save the result
  let mut masked = image;
  mask.apply_to_image(&mut masked);
  masked.write("out/masked.png", None).expect("Failed to save masked image");
}
