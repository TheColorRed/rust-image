use crate::Image;
use crate::fs::mkdirp;
use crate::fs::path::dirname;
use crate::fs::writer_options::WriterOptions;
use std::fs::write;
use turbojpeg::PixelFormat::RGB;
use turbojpeg::compress;

/// Writes the image data to a JPEG file
pub fn write_jpg(p_file: impl Into<String>, p_image: &Image, p_options: &Option<WriterOptions>) -> Result<(), String> {
  let p_file = p_file.into();
  let dir = dirname(p_file.as_str());
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));
  // File::create(file.as_str()).map_err(|e| e.to_string())?;
  let quality = match p_options {
    Some(o) => o.quality,
    None => 100,
  };
  println!("JPEG Quality set to {}", quality);

  let (width, height) = p_image.dimensions::<u32>();

  // Convert our RGBA image to an RGB buffer (JPEG doesn't support alpha)
  let rgb_pixels = p_image.rgb();

  // Build a turbojpeg Image<&[u8]> describing our RGB pixels
  let tj_image = turbojpeg::Image {
    pixels: &rgb_pixels[..],
    width: width as usize,
    pitch: (width as usize) * 3, // 3 bytes per pixel for RGB
    height: height as usize,
    format: RGB,
  };

  // Compress into JPEG using TurboJPEG
  let jpeg_data = compress(tj_image, quality as i32, turbojpeg::Subsamp::Sub2x2).map_err(|e| e.to_string())?;
  write(p_file.as_str(), &jpeg_data).map_err(|e| e.to_string())
}
