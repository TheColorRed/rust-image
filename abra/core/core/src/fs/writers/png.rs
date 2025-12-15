use crate::Image;
use crate::fs::mkdirp;
use crate::fs::path::dirname;
use crate::fs::writer_options::WriterOptions;

use png::ColorType::Rgba;
use png::Encoder;
use std::fs::File;

/// Writes the image data to a PNG file
pub fn write_png(file: impl Into<String>, image: &Image, options: &Option<WriterOptions>) -> Result<(), String> {
  let file = file.into();
  let dir = dirname(&file);
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));
  let file = File::create(file).map_err(|e| e.to_string())?;
  let (width, height) = image.dimensions();
  let mut encoder = Encoder::new(file, width, height);

  let channels = 4; // Always use RGBA

  encoder.set_color(Rgba);
  encoder.set_depth(png::BitDepth::Eight);

  // Set compression level based on quality (higher quality = less compression for speed)
  if let Some(opts) = options {
    let compression = if opts.quality > 75 {
      png::Compression::Fastest
    } else if opts.quality > 25 {
      png::Compression::Balanced
    } else {
      png::Compression::High
    };
    println!("PNG Compression level set to {:?}", compression);
    encoder.set_compression(compression);
  } else {
    encoder.set_compression(png::Compression::default());
    println!("PNG Compression level set to Balanced");
  }

  let mut writer = encoder.write_header().unwrap();
  if channels == 4 {
    let pixels = image.rgba();
    writer.write_image_data(pixels).unwrap();
  } else {
    let pixels = image.rgb();
    writer.write_image_data(&pixels).unwrap();
  }

  Ok(())
}
