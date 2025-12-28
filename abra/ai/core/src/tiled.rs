//! Tiled inference utilities for processing large images.
//!
//! Many AI models have memory constraints or work best with fixed tile sizes.
//! This module provides utilities for splitting images into tiles, processing
//! them individually, and blending the results back together.

use abra_core::Image;

use rayon::prelude::*;

/// Configuration for tiled image processing.
#[derive(Clone, Debug)]
pub struct TileConfig {
  /// Size of each tile (square).
  pub tile_size: u32,
  /// Overlap between adjacent tiles for blending.
  pub overlap: u32,
  /// Scale factor of output relative to input (1.0 for same size, 2.0 for 2x upscale).
  pub scale_factor: f32,
}

impl Default for TileConfig {
  fn default() -> Self {
    Self {
      tile_size: 256,
      overlap: 64,
      scale_factor: 1.0,
    }
  }
}

impl TileConfig {
  /// Creates a new tile config.
  pub fn new(tile_size: u32, overlap: u32) -> Self {
    Self {
      tile_size,
      overlap,
      scale_factor: 1.0,
    }
  }

  /// Sets the tile size.
  pub fn with_tile_size(mut self, size: u32) -> Self {
    self.tile_size = size;
    self
  }

  /// Sets the overlap.
  pub fn with_overlap(mut self, overlap: u32) -> Self {
    self.overlap = overlap;
    self
  }

  /// Sets the scale factor.
  pub fn with_scale_factor(mut self, scale: f32) -> Self {
    self.scale_factor = scale;
    self
  }

  /// Returns the stride (tile_size - overlap).
  pub fn stride(&self) -> u32 {
    self.tile_size.saturating_sub(self.overlap)
  }
}

/// Information about a single tile to be processed.
#[derive(Clone, Debug)]
pub struct TileInfo {
  /// X position in the source image.
  pub x: u32,
  /// Y position in the source image.
  pub y: u32,
  /// Width of the tile.
  pub width: u32,
  /// Height of the tile.
  pub height: u32,
  /// Tile index (0-based).
  pub index: u32,
  /// Total number of tiles.
  pub total: u32,
}

/// Generates tile information for processing an image.
///
/// Returns an iterator of `TileInfo` structs describing each tile's position.
pub fn generate_tiles(image_width: u32, image_height: u32, config: &TileConfig) -> Vec<TileInfo> {
  let tile_size = config.tile_size;
  let stride = config.stride();

  let tiles_x = ((image_width + stride - 1) / stride).max(1);
  let tiles_y = ((image_height + stride - 1) / stride).max(1);
  let total = tiles_x * tiles_y;

  let mut tiles = Vec::with_capacity(total as usize);
  let mut index = 0;

  for ty in 0..tiles_y {
    for tx in 0..tiles_x {
      let mut x = tx * stride;
      let mut y = ty * stride;

      // Align last tile to image edge to avoid small tiles
      if x + tile_size > image_width && image_width > tile_size {
        x = image_width - tile_size;
      }
      if y + tile_size > image_height && image_height > tile_size {
        y = image_height - tile_size;
      }

      let width = tile_size.min(image_width - x);
      let height = tile_size.min(image_height - y);

      tiles.push(TileInfo {
        x,
        y,
        width,
        height,
        index,
        total,
      });
      index += 1;
    }
  }

  tiles
}

/// Accumulator for blending overlapping tile outputs using weighted averaging.
///
/// This handles the common pattern of processing tiles with overlap and
/// averaging the overlapping regions to produce smooth results.
pub struct TileAccumulator {
  width: u32,
  height: u32,
  sum_r: Vec<f32>,
  sum_g: Vec<f32>,
  sum_b: Vec<f32>,
  weights: Vec<f32>,
}

impl TileAccumulator {
  /// Creates a new accumulator for the given output dimensions.
  pub fn new(width: u32, height: u32) -> Self {
    let num_pixels = (width * height) as usize;
    Self {
      width,
      height,
      sum_r: vec![0.0; num_pixels],
      sum_g: vec![0.0; num_pixels],
      sum_b: vec![0.0; num_pixels],
      weights: vec![0.0; num_pixels],
    }
  }

  /// Accumulates a tile's NCHW output data at the given position.
  ///
  /// # Arguments
  ///
  /// - `x`: Destination X position
  /// - `y`: Destination Y position
  /// - `tile_width`: Width of the tile output
  /// - `tile_height`: Height of the tile output
  /// - `data`: Float data in NCHW layout [R..., G..., B...]
  pub fn accumulate(&mut self, x: u32, y: u32, tile_width: u32, tile_height: u32, data: &[f32]) {
    let hw = (tile_width * tile_height) as usize;

    for py in 0..tile_height {
      for px in 0..tile_width {
        let dest_x = x + px;
        let dest_y = y + py;

        if dest_x < self.width && dest_y < self.height {
          let dest_idx = (dest_y * self.width + dest_x) as usize;
          let src_idx = (py * tile_width + px) as usize;

          // NCHW layout: R at [0..hw], G at [hw..2*hw], B at [2*hw..3*hw]
          self.sum_r[dest_idx] += data.get(src_idx).copied().unwrap_or(0.0);
          self.sum_g[dest_idx] += data.get(hw + src_idx).copied().unwrap_or(0.0);
          self.sum_b[dest_idx] += data.get(2 * hw + src_idx).copied().unwrap_or(0.0);
          self.weights[dest_idx] += 1.0;
        }
      }
    }
  }

  /// Finalizes the accumulation and produces the output image.
  ///
  /// Averages all accumulated values and clamps to [0, 1] before converting to u8.
  pub fn finalize(self) -> Image {
    let num_pixels = (self.width * self.height) as usize;
    let mut rgba_data = vec![0u8; num_pixels * 4];

    for i in 0..num_pixels {
      let w = self.weights[i];
      if w > 0.0 {
        let r = (self.sum_r[i] / w).clamp(0.0, 1.0);
        let g = (self.sum_g[i] / w).clamp(0.0, 1.0);
        let b = (self.sum_b[i] / w).clamp(0.0, 1.0);

        rgba_data[i * 4] = (r * 255.0) as u8;
        rgba_data[i * 4 + 1] = (g * 255.0) as u8;
        rgba_data[i * 4 + 2] = (b * 255.0) as u8;
        rgba_data[i * 4 + 3] = 255;
      }
    }

    let mut image = Image::new(self.width, self.height);
    image.set_new_pixels(&rgba_data, self.width, self.height);
    image
  }
}

impl TileAccumulator {
  /// Merge another accumulator into this one (element-wise).
  pub fn merge(&mut self, other: TileAccumulator) {
    debug_assert_eq!(self.width, other.width);
    debug_assert_eq!(self.height, other.height);
    for i in 0..self.sum_r.len() {
      self.sum_r[i] += other.sum_r[i];
      self.sum_g[i] += other.sum_g[i];
      self.sum_b[i] += other.sum_b[i];
      self.weights[i] += other.weights[i];
    }
  }
}

/// Process tiles using the provided `process_tile` callback.
///
/// The `process_tile` callback is given the `TileInfo` and a mutable scratch
/// buffer (NCHW float layout) sized to hold the tile's output and should fill
/// the buffer with R,G,B channels packed as `[R..., G..., B...]`.
///
/// The implementation uses Rayon for parallel processing when the `parallel`
/// feature is enabled and falls back to a sequential implementation otherwise.
pub fn process_tiles<F>(image: &Image, config: &TileConfig, process_tile: F) -> Image
where
  F: Fn(&TileInfo, &mut [f32]) + Sync + Send,
{
  let (width, height) = image.dimensions::<u32>();
  let tiles = generate_tiles(width, height, config);
  let out_width = ((width as f32) * config.scale_factor).round() as u32;
  let out_height = ((height as f32) * config.scale_factor).round() as u32;

  let accs = tiles
    .par_iter()
    .map(|tile| {
      let tile_out_w = ((tile.width as f32) * config.scale_factor).round() as u32;
      let tile_out_h = ((tile.height as f32) * config.scale_factor).round() as u32;
      let buf_len = (3 * tile_out_w * tile_out_h) as usize;
      let mut buf = vec![0f32; buf_len];
      process_tile(tile, &mut buf);
      let mut local_acc = TileAccumulator::new(out_width, out_height);
      let dest_x = ((tile.x as f32) * config.scale_factor).round() as u32;
      let dest_y = ((tile.y as f32) * config.scale_factor).round() as u32;
      local_acc.accumulate(dest_x, dest_y, tile_out_w, tile_out_h, &buf);
      local_acc
    })
    .reduce_with(|mut a, b| {
      a.merge(b);
      a
    })
    .unwrap_or_else(|| TileAccumulator::new(out_width, out_height));

  accs.finalize()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn dummy_process(tile: &TileInfo, buf: &mut [f32]) {
    // fill with gradient based on tile index to make outputs deterministic
    let hw = (buf.len() / 3) as usize;
    for i in 0..hw {
      buf[i] = (tile.index as f32) / (tile.total as f32); // R
      buf[hw + i] = 0.0; // G
      buf[2 * hw + i] = 0.0; // B
    }
  }

  #[test]
  fn test_process_tiles_produces_expected_size() {
    let img = Image::new(100, 80);
    let config = TileConfig {
      tile_size: 32,
      overlap: 8,
      scale_factor: 1.0,
    };
    let out = process_tiles(&img, &config, dummy_process);
    let (w, h) = out.dimensions::<u32>();
    assert_eq!(w, 100);
    assert_eq!(h, 80);
  }

  #[test]
  fn test_process_tiles_deterministic() {
    let img = Image::new(64, 48);
    let config = TileConfig {
      tile_size: 32,
      overlap: 8,
      scale_factor: 1.0,
    };

    let a = process_tiles(&img, &config, dummy_process);
    let b = process_tiles(&img, &config, dummy_process);

    assert_eq!(a.to_rgba_vec(), b.to_rgba_vec());
  }
}
