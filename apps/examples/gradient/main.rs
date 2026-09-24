use abra::abra_core::{ColorStop, blend};
use abra::canvas::prelude::*;
use abra::drawing::prelude::*;
use abra::prelude::*;
use abra::typography::prelude::FontLoader;

const OUT_FILE: &str = "out/gradient.png";

// const BACKGROUND_IMAGE: &str = "assets/nude/black-hair-beauty-long-hair-leg.jpg";
const BACKGROUND_IMAGE: &str = "assets/nude/gravure-idol-black-hair-chest.jpg";

pub fn main() {
  let loader = FontLoader::find_fonts("C:/Users/untun/AppData/Local/Microsoft/Windows/Fonts/**/*.ttf");
  let font = loader.load("Montserrat").unwrap();

  // Create a new canvas for the gradient example and add a background image.
  let canvas = Canvas::new("Gradient");
  let bg_image = canvas.add_layer_from_path("background", BACKGROUND_IMAGE, None);

  // Create a vertical gradient color for the overlay.
  let gradient_color = Gradient::new(vec![
    ColorStop::new(Color::tan(), 0.0),
    ColorStop::new(Color::purple(), 0.5),
    ColorStop::new(Color::blue(), 1.0),
  ])
  .with_direction(Path::line((0, 0), (0, bg_image.dimensions().1)));

  // Fill an area the size of the background image with the gradient color.
  let area = Area::new_from_image(&bg_image.as_image());
  let filled_image = fill(&area, &gradient_color);

  // Add the filled gradient image as a new layer on the canvas.
  let options = NewLayerOptions::new().with_opacity(0.6);
  canvas.add_layer_from_image("gradient", filled_image, options);

  let text = font.text("GRADIENT").with_size(200).with_weight(900).with_fill(gradient_color.clone().with_direction(0));

  let p_options = NewLayerOptions::new().with_opacity(1.0);
  let text_layer = canvas.add_layer_from_image("text", text, p_options);
  text_layer.set_blend_mode(blend::multiply);
  let (width, height) = bg_image.dimensions();
  text_layer.transform().rotate(LineSegment::new((0, 0), (width, height)).degrees(), None);

  // Save the final canvas with the gradient overlay applied.
  canvas.save(OUT_FILE, None);
}
