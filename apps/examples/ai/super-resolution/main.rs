use abra::prelude::*;
use abra_super_resolution::prelude::*;

const FILE: &str = "assets/kelsey.jpg";

fn main() {
  // Load image
  let image = Image::new_from_path(FILE);
  // let (width, height) = image.dimensions::<u32>();

  // Process with default control params
  let output = SuperResolution::load("SCUNet-GAN").process(&image);

  // resize(&mut output, width, height, None);
  // color::auto_color(&mut output, None);

  output.save("out/enhanced.png", None);
  println!("✅ Saved output to out/enhanced.png");
}
