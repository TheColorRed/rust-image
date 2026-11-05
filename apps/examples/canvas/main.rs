use abra::canvas::prelude::*;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const OUT_FILE: &str = "assets/out/canvas.png";

const BACKGROUND_IMAGE: &str = "assets/nude/gravure-idol-black-hair-chest.jpg";
const FOREGROUND_IMAGE: &str = "assets/kelsey.jpg";

pub fn main() {
  #[cfg(feature = "dhat-heap")]
  let _profiler = dhat::Profiler::new_heap();

  let canvas = Canvas::new("Canvas Example");
  let bg_layer = canvas.add_layer_from_path("background", BACKGROUND_IMAGE, None);
  let fg_layer = canvas.add_layer_from_path("foreground", FOREGROUND_IMAGE, None);

  // Save the final canvas with both layers.
  canvas.save(OUT_FILE, None);
}
