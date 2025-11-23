use crate::Image;
use crate::fs::mkdirp;
use crate::fs::path::dirname;
use crate::fs::writer_options::WriterOptions;
use std::fs::write;
use turbojpeg::PixelFormat::RGB;
use turbojpeg::compress;

/// Writes the image data to a JPEG file
pub fn write_jpg(file: impl Into<String>, image: &Image, options: &Option<WriterOptions>) -> Result<(), String> {
  let file = file.into();
  let dir = dirname(file.as_str());
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));
  // File::create(file.as_str()).map_err(|e| e.to_string())?;
  let quality = match options {
    Some(o) => o.quality,
    None => 100,
  };
  println!("JPEG Quality set to {}", quality);

  let (width, height) = image.dimensions::<u32>();
  let image = turbojpeg::Image::mandelbrot(width as usize, height as usize, RGB);
  let jpeg_data = compress(image.as_deref(), quality as i32, turbojpeg::Subsamp::Sub2x2).map_err(|e| e.to_string())?;
  write(file.as_str(), &jpeg_data).map_err(|e| e.to_string())
}
