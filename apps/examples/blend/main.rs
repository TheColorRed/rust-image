use abra::abra_core::blend;
use abra::prelude::*;

const OVERLAY_FILE: &str = "assets/bikini.jpg";
const BASE_FILE: &str = "assets/34KK-breasts.webp";
const OUT_FILE: &str = "out/blend.png";

pub fn main() {
  let mut base = Image::read(BASE_FILE).expect("Failed to load base image");
  let overlay = Image::read(OVERLAY_FILE).expect("Failed to load overlay image");

  let start = std::time::Instant::now();
  blend::blend(
    &mut base,
    &overlay,
    blend::BlendOptions {
      offset: Point::default(),
      opacity: 1.0,
      mode: blend::multiply,
    },
  );
  println!("Blend Time: {:?}", start.elapsed());

  base.write(OUT_FILE, None).expect("Failed to save blended image");
}
