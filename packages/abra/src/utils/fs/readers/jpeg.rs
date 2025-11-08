use jpeg_decoder as jpeg;

use crate::utils::fs::file_info::FileInfo;
use crate::Channels;
use std::fs::File;
use std::io::BufReader;

/// Reads a JPEG file and returns the image data
pub fn read_jpg(file: &str) -> Result<FileInfo, String> {
  let file = File::open(file).map_err(|e| e.to_string())?;
  let mut decoder = jpeg::Decoder::new(BufReader::new(file));

  let pixels = decoder.decode().expect("error decoding jpeg");
  let metadata = decoder.info().unwrap();

  let width = metadata.width as u32;
  let height = metadata.height as u32;

  let info = FileInfo::new(width, height, Channels::RGB, pixels);

  Ok(info)
}
