use image_webp as webp;

use crate::Channels;
use crate::utils::fs::file_info::FileInfo;
use std::fs::File;
use std::io::BufReader;

/// Reads a WebP file and returns the image data
pub fn read_webp(file: &str) -> Result<FileInfo, String> {
  let file = File::open(file).map_err(|e| e.to_string())?;
  let reader = BufReader::new(file);
  let mut decoder = webp::WebPDecoder::new(reader).unwrap();

  let dim = decoder.dimensions();
  let channels = if decoder.has_alpha() { Channels::RGBA } else { Channels::RGB };
  // create a u8 slice to hold the decoded image
  let mut pixels = vec![0; (dim.0 * dim.1 * channels as u32) as usize];
  decoder.read_image(&mut pixels).unwrap();

  let info = FileInfo::new(dim.0, dim.1, channels, pixels);

  Ok(info)
}
