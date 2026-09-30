use std::fs::{self, read};

use resvg::tiny_skia;
use resvg::usvg::{self, Options};

use crate::Channels;
use crate::Image;

/// Reads an SVG file and returns the image data
pub fn read_svg(p_file: impl Into<String>) -> Result<Image, String> {
  let p_file = p_file.into();
  let p_file = p_file.as_str();
  let tree = {
    let mut opt = Options::default();
    // Get file's absolute directory.
    opt.resources_dir = fs::canonicalize(p_file).ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
    opt.fontdb_mut().load_system_fonts();
    let svg_data = read(p_file).unwrap();
    usvg::Tree::from_data(&svg_data, &opt).unwrap()
  };

  let pix_map_size = tree.size().to_int_size();
  let mut pix_map = tiny_skia::Pixmap::new(pix_map_size.width(), pix_map_size.height()).unwrap();
  resvg::render(&tree, tiny_skia::Transform::default(), &mut pix_map.as_mut());

  let pixels = pix_map
    .pixels()
    .to_vec()
    .iter()
    .flat_map(|p| {
      let r = p.red();
      let g = p.green();
      let b = p.blue();
      let a = p.alpha();
      vec![r, g, b, a]
    })
    .collect::<Vec<u8>>();

  Ok(Image::new_from_pixels(pix_map_size.width(), pix_map_size.height(), pixels, Channels::RGBA))
}
