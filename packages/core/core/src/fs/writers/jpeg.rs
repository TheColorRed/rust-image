use crate::fs::mkdirp;
use crate::fs::path::dirname;
use crate::fs::writer_options::WriterOptions;
use crate::image::Image;
use jpeg::ColorType::Rgba;
use jpeg_encoder as jpeg;
use std::fs::File;

/// Writes the image data to a JPEG file
pub fn write_jpg(file: &str, image: &Image, options: &Option<WriterOptions>) -> Result<(), String> {
  let dir = dirname(file);
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));
  let file = File::create(file).map_err(|e| e.to_string())?;
  let (width, height) = image.dimensions::<u32>();
  let quality = match options {
    Some(o) => o.quality,
    None => 100,
  };
  println!("JPEG Quality set to {}", quality);

  let encoder = jpeg::Encoder::new(file, quality);
  let pixels = image.rgba();

  encoder
    .encode(&pixels, width as u16, height as u16, Rgba)
    .expect("error encoding jpeg");

  Ok(())
}
