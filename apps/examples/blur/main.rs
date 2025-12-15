use std::error::Error;

use abra::adjustments::prelude::*;
use abra::prelude::*;
use abra::transform::prelude::*;
use abra_super_resolution::prelude::*;

const FILE: &str = "assets/kelsey.jpg";

fn main() -> Result<(), Box<dyn Error>> {
  // Load image
  let image = Image::new_from_path(FILE);
  let (width, height) = image.dimensions::<u32>();

  // Process with default control params
  let mut output = SuperResolution::load("UltraZoom-2X-Ctrl").process(&image);

  resize(&mut output, width, height, None);
  color::auto_color(&mut output, None);

  output.save("out/enhanced-2x.png", None);
  println!("✅ Saved output to out/enhanced-2x.png");

  Ok(())
}
