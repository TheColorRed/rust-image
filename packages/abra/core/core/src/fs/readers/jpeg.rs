use std::fs::read;

use turbojpeg::PixelFormat::RGB as rgb;
use turbojpeg::decompress;

use crate::Channels;
use crate::Image;

/// Reads a JPEG file and returns the image data.
/// - `p_file`: the path to the JPEG file to read.
pub fn read_jpg(p_file: impl Into<String>) -> Result<Image, String> {
  let jpeg_data = read(p_file.into()).map_err(|e| e.to_string())?;
  let data = decompress(&jpeg_data, rgb).map_err(|e| e.to_string())?;
  let info = Image::new_from_pixels(data.width as u32, data.height as u32, data.pixels, Channels::RGB);
  Ok(info)
}
