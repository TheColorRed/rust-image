use crate::{utils::fs::file_info::FileInfo, Channels};
use gif::DecodeOptions;
use std::fs::File;

/// Reads a GIF file and returns the first frame's image data
pub fn read_gif(file: &str) -> Result<FileInfo, String> {
  let file = File::open(file).map_err(|e| e.to_string())?;
  let decoder = DecodeOptions::new();
  let mut decoder = decoder.read_info(file).map_err(|e| e.to_string())?;

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
fn indexed_to_rgba(indexed_data: &[u8], width: u32, height: u32, palette: Option<&[u8]>) -> Result<Vec<u8>, String> {
  let palette = palette.ok_or("GIF has no palette")?;
  let mut rgba = Vec::with_capacity((width * height * 4) as usize);

  for &index in indexed_data {
    let idx = (index as usize) * 3;
    if idx + 3 <= palette.len() {
      rgba.push(palette[idx]); // R
      rgba.push(palette[idx + 1]); // G
      rgba.push(palette[idx + 2]); // B
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
