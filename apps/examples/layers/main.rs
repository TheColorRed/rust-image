#![allow(unused_imports)]
use abra::abra_core::blend;
use abra::adjustments::prelude::*;
use abra::canvas::prelude::*;
use abra::prelude::*;

const BOTTOM_IMAGE: &str = "assets/aletta-ocean.jpg";
// const BOTTOM_IMAGE: &str = "assets/boobs.webp";
const TOP_IMAGE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/layers.png";

const _CANVAS2_BOTTOM_IMAGE: &str = "assets/34KK-breasts.webp";
const _CANVAS2_TOP_IMAGE: &str = "assets/skirt.png";

pub fn main() {
  let canvas_root = Canvas::new("My Canvas Project").add_layer_from_path("Background", "assets/kelsey.jpg", None);

  if let Some(mut layer) = canvas_root.get_layer_by_name("Background") {
    color::auto_color(&mut layer, None);
  }

  canvas_root.save(OUT_FILE, None);
}
