use image_webp as webp;

use crate::Channels;
use crate::fs::file_info::FileInfo;
use std::fs::File;
use std::io::BufReader;

/// Reads a WebP file and returns the image data
pub fn read_webp(file: impl Into<String>) -> Result<FileInfo, String> {
  let file_path = file.into();
  let file = File::open(&file_path).map_err(|e| format!("Failed to open file: {}", e))?;
  // Larger buffer for better IO performance
  let reader = BufReader::with_capacity(1 << 20, file); // 1 MiB

  let mut decoder = webp::WebPDecoder::new(reader).map_err(|e| format!("Failed to create WebP decoder: {:?}", e))?;

  // Set a very high memory limit (1 GiB) to allow large images
  decoder.set_memory_limit(1024 * 1024 * 1024);

  let dim = decoder.dimensions();
  let channels = if decoder.has_alpha() {
    Channels::RGBA
  } else {
    Channels::RGB
  };

  // Use output_buffer_size to get the correct buffer size (this respects the decoder's calculations)
  let buffer_size = decoder
    .output_buffer_size()
    .ok_or_else(|| format!("Image too large to decode: {}x{}", dim.0, dim.1))?;

  let mut pixels = vec![0u8; buffer_size];
  decoder
    .read_image(&mut pixels)
    .map_err(|e| format!("Failed to decode WebP image: {:?}", e))?;

  let info = FileInfo::new(dim.0, dim.1, channels, pixels);

  Ok(info)
}
