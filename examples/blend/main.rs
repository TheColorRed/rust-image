use abra::{Image, blend};

const OVERLAY_FILE: &str = "assets/bikini.jpg";
const BASE_FILE: &str = "assets/34KK-breasts.webp";
const OUT_FILE: &str = "out/blend.png";

pub fn main() {
  let mut base = Image::new_from_path(BASE_FILE);
  let overlay = Image::new_from_path(OVERLAY_FILE);

  let start = std::time::Instant::now();
  blend::blend_images(&mut base, &overlay, blend::divide);
  println!("Blend Time: {:?}", start.elapsed());

  base.save(OUT_FILE, None);
}
