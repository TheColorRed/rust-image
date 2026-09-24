use abra::prelude::*;
use abra_super_resolution::prelude::*;

const FILE: &str = "assets/kelsey.jpg";

fn main() {
  // Load image
  let image = Image::read(FILE).expect("Failed to load image");
  // let (width, height) = image.dimensions::<u32>();

  // Process with default control params
  let output = SuperResolution::load("SCUNet-GAN").process(&image);

  // resize(&mut output, width, height, None);
  // color::auto_color().apply(&mut output);

  output.write("out/enhanced.png", None).expect("Failed to save image");
  println!("✅ Saved output to out/enhanced.png");
}
