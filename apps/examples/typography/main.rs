#![allow(unused_imports)]
use abra::abra_core::{Resolution, blend};
use abra::canvas::prelude::*;
use abra::prelude::*;
use abra::typography::prelude::{Font, FontLoader, TextAlign, TextSize};

const BACKGROUND_IMAGE: &str = "assets/kelsey.jpg";
const TEXT: &str = "Abra";
const TEXT_SIZE: u32 = 50;
const OUT_FILE: &str = "out/typography.png";

pub fn main() {
  let fill = Gradient::harmony(Color::ruby(), Harmony::Complementary); //.with_direction(90);

  // Background canvas: the photo on its own layer.
  let background = Canvas::new_from_path("Background", BACKGROUND_IMAGE, None);
  let (width, height): (u32, u32) = background.dimensions();

  // let mut system_fonts = FontLoader::load_system_fonts();
  let loader = FontLoader::find_fonts("C:/Users/untun/AppData/Local/Microsoft/Windows/Fonts/**/*.ttf");
  let font = loader.load("Montserrat").unwrap();

  // Create a blank canvas for the text layer.
  let text_canvas = Canvas::new_from_unit("Text", CanvasUnit::Pixels(width, height, Resolution::LINE_ART));
  let text = font
    .text(TEXT)
    .with_size(TextSize::points(TEXT_SIZE))
    .with_width(400)
    .with_height(height as f32 * 0.8)
    .with_fill(fill.clone())
    .with_alignment(TextAlign::Center)
    .with_letter_spacing(40)
    .with_weight(800);
  text_canvas.add_layer_from_image("Title", text, None);
  text_canvas.set_blend_mode(BlendMode::Multiply);

  // Root canvas: a canvas's own layers always render above its child canvases, so both the
  // background and text must be added as child canvases (in bottom-to-top order) to stack correctly.
  let root = Canvas::new_blank("Composition", width, height);
  root.add_canvas(background, AddCanvasOptions::new().with_anchor(Anchor::Center));
  root.add_canvas(text_canvas, AddCanvasOptions::new().with_anchor(Anchor::Center));

  root.resample(CanvasUnit::Points(width * 2, height * 2, Resolution::SCREEN));

  root.save(OUT_FILE, None);
}
