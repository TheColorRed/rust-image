use abra::prelude::*;
use abra::tools::prelude::AtlasDimension::Fixed;
use abra::tools::prelude::*;
use abra::tools::prelude::{skin_smooth, skin_tan, skin_tone};
use abra_body_segmentation::BodySegmentation;

fn main() {
  // Read the original image and prepare for skin effects.
  let image_orig = Image::read("assets/nude/freckles-blonde.jpeg").expect("Failed to read image");
  let segmenter = BodySegmentation::load("abra/ai/body-segmentation/models/selfie_multiclass_256x256.onnx")
    .expect("Failed to load the skin model");
  let mask = segmenter.process(&image_orig).expect("Failed to segment skin");

  // Apply skin effects to a clone of the original image.
  let mut atlas_smooth = atlas().with_dimensions(5);
  let mut effects = image_orig.clone();
  atlas_smooth.append(image_orig.clone());
  for amount in [0.2, 0.4, 0.6, 0.8, 1.0, 2.0, 3.0, 4.0, 5.0].into_iter() {
    skin_smooth(amount).with_mask(&mask).apply(&mut effects);
    atlas_smooth.append(effects.clone());
  }
  atlas_smooth.create().write("out/skin-smooth.png", None).expect("Failed to write image");

  // Create a side-by-side comparison of the original and the effects.
  let mut atlas_tan = atlas().with_dimensions(4);
  atlas_tan.append(image_orig.clone());
  let mut effects = image_orig.clone();
  let colors = [
    Color::tan(),
    Color::bronze(),
    Color::gold(),
    Color::golden(),
    Color::brown(),
    Color::dark_brown(),
    Color::light_brown(),
  ];
  for color in colors.into_iter() {
    skin_tan(color).with_mask(&mask).apply(&mut effects);
    atlas_tan.append(effects.clone());
  }
  atlas_tan.create().write("out/skin-tan.png", None).expect("Failed to write image");

  let mut atlas = atlas().with_dimensions(Fixed(4, 2)).with_trim(true);
  atlas.append(image_orig.clone());
  for amount in [100, 200, 400, -100, -200, -400].into_iter() {
    let mut toned = image_orig.clone();
    skin_tone(amount).with_mask(&mask).apply(&mut toned);
    atlas.append(toned);
    if amount == 400 {
      atlas.append(image_orig.clone());
    }
  }
  atlas.create().write("out/skin-tone.png", None).expect("Failed to write skin tone comparison");
}
