use std::{fs::File, io::BufWriter};

use crate::Image;
use crate::fs::writer_options::WriterOptions;
use image_webp as webp;
use webp::ColorType::Rgba8;

/// Writes the image data to a WebP file.
///
/// With `p_options`, the image is encoded lossy at its `quality` (0 to 100), which makes a much smaller file than the
/// lossless encoding. Without options it is encoded lossless, so every pixel is kept as it is.
pub fn write_webp(
  p_file: impl Into<String>, p_img: &Image, p_options: &Option<WriterOptions>,
) -> Result<(), String> {
  let p_file = p_file.into();
  let (width, height) = p_img.dimensions::<u32>();

  if let Some(options) = p_options {
    // `encode` panics when libwebp cannot encode the image (a side over 16383 pixels, or too little memory), and a panic
    // while the image is locked poisons the lock for good. `encode_simple` returns the failure instead.
    let encoded = libwebp::Encoder::from_rgba(p_img.rgba(), width, height)
      .encode_simple(false, options.quality.min(100) as f32)
      .map_err(|error| format!("WebP encoding failed: {error:?}"))?;
    return std::fs::write(&p_file, &*encoded).map_err(|e| e.to_string());
  }

  let p_file = File::create(p_file).map_err(|e| e.to_string())?;
  let writer = BufWriter::new(p_file);
  let encoder = webp::WebPEncoder::new(writer);
  encoder.encode(p_img.rgba(), width, height, Rgba8).map_err(|e| e.to_string())
}
