use std::{fs::File, io::BufWriter};

use crate::Image;
use crate::fs::mkdirp;
use crate::fs::path::dirname;
use image_webp as webp;
use webp::ColorType::Rgba8;

/// Writes the image data to a WebP file
pub fn write_webp(file: impl Into<String>, img: &Image) -> Result<(), String> {
  let file = file.into();
  let dir = dirname(&file);
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));
  let file = File::create(file).map_err(|e| e.to_string())?;
  let writer = BufWriter::new(file);
  let encoder = webp::WebPEncoder::new(writer);
  let pixels = img.rgba();
  let (width, height) = img.dimensions();

  encoder
    .encode(pixels, width, height, Rgba8)
    .expect("error encoding webp");

  Ok(())
}
