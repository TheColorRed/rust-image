use crate::Image;
use crate::fs::mkdirp;
use crate::fs::path::dirname;
use crate::fs::writer_options::WriterOptions;
use gif::{Encoder, Frame, Repeat};
use std::fs::File;

/// Writes the image data to a GIF file
pub fn write_gif(p_file: impl Into<String>, p_image: &Image, p_options: &Option<WriterOptions>) -> Result<(), String> {
  let p_file = p_file.into();
  let dir = dirname(&p_file);
  mkdirp(&dir).unwrap_or_else(|_| panic!("Error creating directory {}", &dir));

  let file_handle = File::create(p_file).map_err(|e| e.to_string())?;
  let (width, height) = p_image.dimensions::<u16>();

  let mut encoder = Encoder::new(file_handle, width, height, &[]).map_err(|e| e.to_string())?;

  // Set repeat to loop infinitely by default
  encoder.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;

  // Get the RGBA pixel data
  let rgba_pixels = p_image.rgba();

  // Convert RGBA to indexed color (palette-based)
  let (indexed_pixels, palette) = rgba_to_indexed(&rgba_pixels)?;

  // Create a frame with the indexed data
  let mut frame = Frame::default();
  frame.width = width;
  frame.height = height;
  frame.buffer = std::borrow::Cow::Owned(indexed_pixels);
  frame.palette = Some(palette);

  // Set delay if options are provided
  if let Some(opts) = p_options {
    // Assume quality is used for frame delay in centiseconds (1-100)
    // Higher quality = shorter delay for animation
    frame.delay = ((100 - opts.quality) / 10).max(1).min(100) as u16;
  } else {
    frame.delay = 10; // Default 100ms delay
  }

  encoder.write_frame(&frame).map_err(|e| e.to_string())?;

  println!("GIF written successfully");
  Ok(())
}

/// Converts RGBA format to indexed color (palette-based) format using a simple approach
fn rgba_to_indexed(p_rgba_pixels: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
  // For simplicity, we'll use a basic color quantization approach
  // This creates a 256-color palette from the RGBA data

  let mut palette_map = std::collections::HashMap::new();
  let mut palette = Vec::new();
  let mut indexed_data = Vec::new();

  let num_pixels = p_rgba_pixels.len() / 4;

  for i in 0..num_pixels {
    let offset = i * 4;
    let r = p_rgba_pixels[offset];
    let g = p_rgba_pixels[offset + 1];
    let b = p_rgba_pixels[offset + 2];

    // Create a color key (ignore alpha for palette indexing)
    let color_key = (r, g, b);

    let palette_index = if let Some(&idx) = palette_map.get(&color_key) {
      idx
    } else {
      if palette.len() >= 768 {
        // Palette full (256 colors * 3 bytes), use nearest color
        find_nearest_color(r, g, b, &palette) as u8
      } else {
        let idx = (palette.len() / 3) as u8;
        palette.push(r);
        palette.push(g);
        palette.push(b);
        palette_map.insert(color_key, idx);
        idx
      }
    };

    indexed_data.push(palette_index);
  }

  // Pad palette to 256 colors if needed
  while palette.len() < 768 {
    palette.push(0);
  }

  Ok((indexed_data, palette))
}

/// Finds the nearest color in the palette to the given RGB values
fn find_nearest_color(p_r: u8, p_g: u8, p_b: u8, p_palette: &[u8]) -> usize {
  let mut min_distance = i32::MAX;
  let mut nearest_idx = 0;

  for i in (0..p_palette.len()).step_by(3) {
    let pr = p_palette[i] as i32;
    let pg = p_palette[i + 1] as i32;
    let pb = p_palette[i + 2] as i32;

    let dr = (p_r as i32) - pr;
    let dg = (p_g as i32) - pg;
    let db = (p_b as i32) - pb;

    let distance = dr * dr + dg * dg + db * db;

    if distance < min_distance {
      min_distance = distance;
      nearest_idx = i / 3;
    }
  }

  nearest_idx
}
