use abra::adjustments::prelude::*;
use abra::canvas::prelude::*;
use abra::filters::prelude::*;

const FILE: &str = "assets/kelsey.jpg";
// const FILE: &str = "assets/bikini.jpg";
// const OUT_FILE: &str = "out/blur.png";

fn main() {
  let canvas = Canvas::new_from_path("Kelsey", FILE, None);
  if let Some(mut layer) = canvas.get_layer_by_name("Kelsey") {
    noise::despeckle(&mut layer, None);
    color::auto_tone(&mut layer, None);
    color::auto_color(&mut layer, None);
    // blur::gaussian_blur(&mut layer, 20, None);
    canvas.save("out/focus-blur.png", None);
  }
}
