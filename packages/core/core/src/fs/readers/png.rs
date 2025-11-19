use crate::{Channels, fs::file_info::FileInfo};
use png::Decoder;
use std::fs::File;
use std::io::BufReader;

/// Reads a PNG file and returns the image data
pub fn read_png(file: &str) -> Result<FileInfo, String> {
  let file = File::open(file).map_err(|e| e.to_string())?;
  // Larger buffer for better IO performance on large PNGs
  let reader = BufReader::with_capacity(1 << 20, file); // 1 MiB
  let decoder = Decoder::new(reader);
  let mut reader = decoder.read_info().unwrap();
  let output_size = reader.output_buffer_size().ok_or("Failed to get buffer size")?;
  let mut buf = vec![0; output_size];
  let info = reader.next_frame(&mut buf).unwrap();
  let bytes = &buf[..info.buffer_size()];

  let width = info.width as u32;
  let height = info.height as u32;
  let pixels = bytes.to_vec();

  let channels = match info.color_type {
    png::ColorType::Rgb => Channels::RGB,
    png::ColorType::Rgba => Channels::RGBA,
    _ => panic!("Unsupported color type"),
  };

  let info = FileInfo::new(width, height, channels, pixels);

  Ok(info)
}
