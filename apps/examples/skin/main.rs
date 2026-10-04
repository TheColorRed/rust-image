// use abra::drawing::prelude::fill;
use abra::filters::prelude::skin::{smooth_skin, tan_skin};
use abra::filters::prelude::*;
use abra::prelude::*;

fn main() {
  // Skin smoothing at three strengths: 1 is the full mask, and 2 and 3 make the smoothing itself stronger.
  for (name, file) in [("aletta", "assets/aletta-ocean.jpg"), ("skirt", "assets/skirt.png")] {
    for amount in [1.0, 2.0, 3.0] {
      let mut image = Image::read(file).expect("Failed to read image");

      smooth_skin(amount).apply(&mut image);
      tan_skin(Color::brown()).apply(&mut image);
      // fill(&image, Color::light_brown()).apply(&mut image);

      image.write(format!("out/skin-{name}-{amount}.png"), None).expect("Failed to write image");
    }
  }
}
