use abra::abra_core::Star;
use abra::adjustments::prelude::*;
use abra::canvas::prelude::*;
use abra::filters::prelude::*;
use abra::options::prelude::*;
use abra::prelude::*;

const FILE: &str = "assets/kelsey.jpg";
// const FILE: &str = "assets/bikini.jpg";
// const OUT_FILE: &str = "out/blur.png";

fn main() {
  let mut image = Image::new_from_path(FILE);

  let canvas = Canvas::new_from_path("Kelsey", FILE, None);
  // .add_layer_from_path("Kelsey", FILE, None);
  if let Some(mut layer) = canvas.get_layer_by_name("Kelsey") {
    noise::despeckle(&mut layer, None);
    color::auto_tone(&mut layer, None);
    color::auto_color(&mut layer, None);
    blur::gaussian_blur(&mut layer, 20, None);
    canvas.save("out/focus-blur.png", None);
    return;
  }

  let area = Star::new().fit((500, 500)).with_feather(50);
  let options = ApplyOptions::new().with_area(area.clone().with_feather(10));
  // levels::contrast(&mut image, 100, None);
  // noise::despeckle(&mut image, None);
  // color::auto_tone(&mut image, None);
  // color::auto_color(&mut image, None);
  // levels::photo_filter_preset(&mut image, FilterType::WarmingDark, 0.25, None);
  // smooth::smooth_skin(&mut image, 0.5, None);
  // levels::saturation(&mut image, 150, None);
  image.save("out/focus-blur.png", None);
}
