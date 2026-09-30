use crate::Image;
use crate::fs::writer_options::WriterOptions;

use png::ColorType::Rgba;
use png::Encoder;
use std::fs::File;

/// Writes the image data to a PNG file
pub fn write_png(p_file: impl Into<String>, p_image: &Image, p_options: &Option<WriterOptions>) -> Result<(), String> {
  let p_file = p_file.into();
  let p_file = File::create(p_file).map_err(|e| e.to_string())?;
  let (width, height) = p_image.dimensions();
  let mut encoder = Encoder::new(p_file, width, height);

  let channels = 4; // Always use RGBA

  encoder.set_color(Rgba);
  encoder.set_depth(png::BitDepth::Eight);

  // Set compression level based on quality (higher quality = less compression for speed)
  if let Some(opts) = p_options {
    let compression = if opts.quality > 75 {
      png::Compression::Fastest
    } else if opts.quality > 25 {
      png::Compression::Balanced
    } else {
      png::Compression::High
    };
    encoder.set_compression(compression);
  } else {
    encoder.set_compression(png::Compression::default());
  }

  let mut writer = encoder.write_header().unwrap();
  if channels == 4 {
    let pixels = p_image.rgba();
    writer.write_image_data(pixels).unwrap();
  } else {
    let pixels = p_image.rgb();
    writer.write_image_data(&pixels).unwrap();
  }

  Ok(())
}
