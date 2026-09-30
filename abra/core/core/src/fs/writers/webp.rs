use std::{fs::File, io::BufWriter};

use crate::Image;
use image_webp as webp;
use webp::ColorType::Rgba8;

/// Writes the image data to a WebP file
pub fn write_webp(p_file: impl Into<String>, p_img: &Image) -> Result<(), String> {
  let p_file = p_file.into();
  let p_file = File::create(p_file).map_err(|e| e.to_string())?;
  let writer = BufWriter::new(p_file);
  let encoder = webp::WebPEncoder::new(writer);
  let pixels = p_img.rgba();
  let (width, height) = p_img.dimensions();

  encoder.encode(pixels, width, height, Rgba8).expect("error encoding webp");

  Ok(())
}
