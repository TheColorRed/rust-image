#![allow(unused_imports)]
use abra::abra_core::blend;
use abra::adjustments::prelude::*;
use abra::canvas::prelude::*;
use abra::prelude::*;
use std::sync::Arc;

const BOTTOM_IMAGE: &str = "assets/aletta-ocean.jpg";
// const BOTTOM_IMAGE: &str = "assets/boobs.webp";
const TOP_IMAGE: &str = "assets/bikini.jpg";
const OUT_FILE: &str = "out/layers.png";

const _CANVAS2_BOTTOM_IMAGE: &str = "assets/34KK-breasts.webp";
const _CANVAS2_TOP_IMAGE: &str = "assets/skirt.png";

pub fn main() {
  {
    // Preload and process the background image before creating the canvas to avoid borrowing issues.
    let mut bg = Image::read("assets/kelsey.jpg").expect("Failed to load background image");
    color::auto_color().apply(&mut bg);

    // Save the processed background image directly for the example.
    bg.write(OUT_FILE, None).expect("Failed to save background image");
  }
}
