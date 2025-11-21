use std::fs::read;

use turbojpeg::PixelFormat::RGB as rgb;
use turbojpeg::decompress;

use crate::Channels;
use crate::fs::file_info::FileInfo;

/// Reads a JPEG file and returns the image data.
/// - `p_file`: the path to the JPEG file to read.
pub fn read_jpg(p_file: impl Into<String>) -> Result<FileInfo, String> {
  let jpeg_data = read(p_file.into()).map_err(|e| e.to_string())?;
  let data = decompress(&jpeg_data, rgb).map_err(|e| e.to_string())?;
  let info = FileInfo::new(data.width as u32, data.height as u32, Channels::RGB, data.pixels);
  Ok(info)
}
