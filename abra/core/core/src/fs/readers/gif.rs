use crate::{Channels, fs::file_info::FileInfo};
use gif::DecodeOptions;
use std::fs::File;
use std::io::BufReader;

/// Reads a GIF file and returns the first frame's image data
pub fn read_gif(p_file: impl Into<String>) -> Result<FileInfo, String> {
  let p_file = p_file.into();
  let p_file = File::open(p_file).map_err(|e| e.to_string())?;
  let decoder = DecodeOptions::new();
  // Wrap in a buffered reader to reduce syscalls
  let reader = BufReader::with_capacity(1 << 20, p_file); // 1 MiB
  let mut decoder = decoder.read_info(reader).map_err(|e| e.to_string())?;

  // Decode the first frame
  let frame = decoder.read_next_frame().map_err(|e| e.to_string())?.ok_or("No frames in GIF")?;

  let width = frame.width as u32;
  let height = frame.height as u32;
  let buffer = frame.buffer.to_vec();

  // Convert indexed color to RGBA
  let pixels = indexed_to_rgba(&buffer, width, height, decoder.global_palette())?;

  let info = FileInfo::new(width, height, Channels::RGBA, pixels);

  Ok(info)
}

/// Converts indexed color (palette-based) format to RGBA format
fn indexed_to_rgba(
  p_indexed_data: &[u8], p_width: u32, p_height: u32, p_palette: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
  let p_palette = p_palette.ok_or("GIF has no palette")?;
  let mut rgba = Vec::with_capacity((p_width * p_height * 4) as usize);

  for &index in p_indexed_data {
    let idx = (index as usize) * 3;
    if idx + 3 <= p_palette.len() {
      rgba.push(p_palette[idx]); // R
      rgba.push(p_palette[idx + 1]); // G
      rgba.push(p_palette[idx + 2]); // B
      rgba.push(255); // A (fully opaque)
    } else {
      // Out of palette bounds, use black
      rgba.push(0);
      rgba.push(0);
      rgba.push(0);
      rgba.push(255);
    }
  }

  Ok(rgba)
}
