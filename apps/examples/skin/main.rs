use abra::drawing::prelude::fill;
use abra::filters::prelude::skin::{skin_smooth, skin_tan, skin_tone};
use abra::filters::prelude::*;
use abra::prelude::*;
use abra_body_segmentation::segment_skin;

fn main() {
  // Read the original image and prepare for skin effects.
  let image_orig = Image::read("assets/nude/freckles-blonde.jpeg").expect("Failed to read image");
  let (width, height) = image_orig.dimensions::<u32>();

  // Apply skin effects to a clone of the original image.
  let mut effects = image_orig.clone();
  let mask = segment_skin(&effects).expect("Failed to segment skin");
  skin_smooth(0.2).with_mask(mask.clone()).apply(&mut effects);
  skin_tan(Color::tan()).with_mask(mask.clone()).apply(&mut effects);

  // Create a new image that is twice the width of the original to place the original and the effects side by side.
  let mut image = Image::new(width * 2, height);
  fill(&image_orig, &image_orig).apply(&mut image);
  fill(&effects, &effects).with_position((width, 0)).apply(&mut image);

  image.write("out/kelsey-tan.png", None).expect("Failed to write image");

  // Compare lighter (-100), original (0), and darker (+100) without adding a tint.
  let mut tones = Image::new(width * 7, height);
  for (index, amount) in [-400, -200, -100, 0, 100, 200, 400].into_iter().enumerate() {
    let mut toned = image_orig.clone();
    skin_tone(amount).with_mask(mask.clone()).apply(&mut toned);
    fill(&toned, &toned).with_position((index as u32 * width, 0)).apply(&mut tones);
  }
  tones.write("out/skin-tone.png", None).expect("Failed to write skin tone comparison");
}
