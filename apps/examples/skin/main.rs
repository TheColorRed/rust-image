use abra::drawing::prelude::fill;
use abra::filters::prelude::skin::{smooth_skin, tan_skin};
use abra::filters::prelude::*;
use abra::prelude::*;
use abra_body_segmentation::segment_skin;

fn main() {
  // Read the original image and prepare for skin effects.
  let image_orig = Image::read("assets/boobs.webp").expect("Failed to read image");
  let (width, height) = image_orig.dimensions::<u32>();

  // Apply skin effects to a clone of the original image.
  let mut effects = image_orig.clone();
  let mask = segment_skin(&effects).expect("Failed to segment skin");
  smooth_skin(0.2).with_mask(mask.clone()).apply(&mut effects);
  tan_skin(Color::tan()).with_mask(mask.clone()).apply(&mut effects);

  // Create a new image that is twice the width of the original to place the original and the effects side by side.
  let mut image = Image::new(width * 2, height);
  fill(&image_orig, &image_orig).apply(&mut image);
  fill(&effects, &effects).with_position((width, 0)).apply(&mut image);

  image.write("out/kelsey-tan.png", None).expect("Failed to write image");

  // for (name, file) in [("aletta", "assets/aletta-ocean.jpg"), ("skirt", "assets/skirt.png")] {
  //   for amount in [1.0, 2.0, 3.0] {
  //     let mut image = Image::read(file).expect("Failed to read image");

  //     // smooth_skin(amount).apply(&mut image);
  //     let mask = segment_skin(&image).expect("Failed to segment skin");
  //     tan_skin(Color::brown()).with_mask(mask).apply(&mut image);
  //     // fill(&image, Color::light_brown()).apply(&mut image);

  //     image.write(format!("out/skin-{name}-{amount}.png"), None).expect("Failed to write image");
  //   }
  // }
}
