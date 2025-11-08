use std::{fs::File, io::BufWriter};

use crate::image::Image;
use crate::path::dirname;
use crate::utils::fs::mkdirp;
use image_webp as webp;
use webp::ColorType::Rgba8;

/// Writes the image data to a WebP file
pub fn write_webp(file: &str, img: &Image) -> Result<(), String> {
  let dir = dirname(file);
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));
  let file = File::create(file).map_err(|e| e.to_string())?;
  let writer = BufWriter::new(file);
  let encoder = webp::WebPEncoder::new(writer);
  let pixels = img.rgba();
  let (width, height) = img.dimensions();

  encoder.encode(&pixels, width, height, Rgba8).expect("error encoding webp");

  Ok(())
}
