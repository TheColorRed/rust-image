//! Tiled inference utilities for processing large images.
//!
//! Many AI models have memory constraints or work best with fixed tile sizes.
//! This module provides utilities for splitting images into tiles, processing
//! them individually, and blending the results back together.

use abra_core::Image;

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
