//! Guided, tiled alpha refinement using an [`AiModel`].

use abra_core::{Image, Rect, ResizeTarget, Size, TransformAlgorithm, transform};
use mask::Mask;

use crate::{AiError, AiModel, AiOutput};

/// Settings for refining a rough foreground mask with a guided model.
#[derive(Clone, Copy, Debug)]
pub struct EdgeMatteOptions {
  /// Width of the unknown band as a share of the photo width.
  unknown_band: f32,
  /// Scale applied to the area around the mask before tiling.
  scale: f32,
  /// Maximum width and height of a model tile.
  tile_size: u32,
  /// Context shared by neighboring tiles.
  overlap: u32,
}

impl Default for EdgeMatteOptions {
  fn default() -> Self {
    Self {
      unknown_band: 0.012,
      scale: 1.5,
      tile_size: 1024,
      overlap: 128,
    }
  }
}

impl EdgeMatteOptions {
  /// Creates the default refinement settings.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the unknown band width as a share of the photo width.
  pub fn with_unknown_band(mut self, width: f32) -> Self {
    self.unknown_band = width;
    self
  }

  /// Sets the scale of the cropped image and guide before tiling.
  pub fn with_scale(mut self, scale: f32) -> Self {
    self.scale = scale;
    self
  }

  /// Sets the maximum width and height of a model tile.
  pub fn with_tile_size(mut self, size: u32) -> Self {
    self.tile_size = size;
    self
  }

  /// Sets the overlap between neighboring tiles.
  pub fn with_overlap(mut self, overlap: u32) -> Self {
    self.overlap = overlap;
    self
  }
}

/// Refines the edge of `rough` with a guided alpha model and returns a photo-sized mask.
///
/// Work stays in Abra types from input to output. Tiles are processed one at a time; only their center regions are copied into one
/// output buffer, avoiding a photo-sized mask allocation for every tile.
/// - `progress`: Receives the share of tiles completed, from `0.0` through `1.0`.
pub fn edge_matte(
  model: &AiModel,
  photo: &Image,
  rough: &Mask,
  options: EdgeMatteOptions,
  mut progress: impl FnMut(f32),
  is_cancelled: impl Fn() -> bool,
) -> Result<Option<Mask>, AiError> {
  if !options.unknown_band.is_finite() || options.unknown_band < 0.0 || !options.scale.is_finite() || options.scale <= 0.0 {
    return Err(AiError::invalid_input("Edge matting scale and unknown band must be finite and nonnegative"));
  }
  if options.tile_size == 0 || options.overlap >= options.tile_size {
    return Err(AiError::invalid_input("Edge matting tile size must be greater than its overlap"));
  }

  let (photo_width, photo_height) = photo.dimensions::<u32>();
  if photo_width == 0 || photo_height == 0 {
    return Err(AiError::invalid_input("Cannot refine an empty picture"));
  }
  let reach = ((photo_width as f32 * options.unknown_band).round() as u32).max(2);
  let grown = rough.dilated(reach);
  let Some(bounds) = grown.bounds(0) else { return Ok(None) };
  let left = bounds.x.floor().max(0.0) as u32;
  let top = bounds.y.floor().max(0.0) as u32;
  let right = bounds.right().ceil().clamp(0.0, photo_width as f32) as u32;
  let bottom = bounds.bottom().ceil().clamp(0.0, photo_height as f32) as u32;
  let (part_width, part_height) = (right.saturating_sub(left), bottom.saturating_sub(top));
  if part_width == 0 || part_height == 0 {
    return Ok(None);
  }
  let part = Rect::new((left, top), (part_width, part_height));

  let unknown = Mask::from_values(
    grown.width(),
    grown.height(),
    grown.values().iter().map(|value| ((u16::from(*value) * 128) / 255) as u8).collect(),
  );
  let trimap = unknown.maximum(&rough.eroded(reach)).cropped(part);
  let width = ((part_width as f32 * options.scale).round() as u32).max(1);
  let height = ((part_height as f32 * options.scale).round() as u32).max(1);
  let picture = transform()
    .with_crop(part)
    .with_size(ResizeTarget::Exact(Size::new(width, height)))
    .with_algorithm(TransformAlgorithm::Bilinear)
    .applied(photo);
  let guide = trimap.resized(width, height, TransformAlgorithm::Bilinear);

  let xs = tile_starts(width, options.tile_size, options.overlap);
  let ys = tile_starts(height, options.tile_size, options.overlap);
  let tile_count = xs.len() * ys.len();
  let trim = options.overlap / 2;
  let mut alpha = vec![0u8; width as usize * height as usize];
  let mut completed = 0usize;
  for (row, &top) in ys.iter().enumerate() {
    for (column, &left) in xs.iter().enumerate() {
      if is_cancelled() {
        return Ok(None);
      }
      let tile_width = options.tile_size.min(width - left);
      let tile_height = options.tile_size.min(height - top);
      let tile = Rect::new((left, top), (tile_width, tile_height));
      let tile_guide = guide.cropped(tile);
      if tile_guide.bounds(0).is_some() {
        let tile_picture = transform().with_crop(tile).applied(&picture);
        let found = match model.run_guided(&tile_picture, Some(&tile_guide), &|_| {})? {
          AiOutput::Mask(mask) => mask,
          _ => return Err(AiError::invalid_input("Edge matting model must return a mask")),
        };
        let cut_left = if column > 0 { trim } else { 0 };
        let cut_top = if row > 0 { trim } else { 0 };
        let cut_right = if column + 1 < xs.len() { trim } else { 0 };
        let cut_bottom = if row + 1 < ys.len() { trim } else { 0 };
        let middle = found.cropped(Rect::new(
          (cut_left, cut_top),
          (tile_width - cut_left - cut_right, tile_height - cut_top - cut_bottom),
        ));
        paste_max(
          &mut alpha,
          width,
          middle.values(),
          middle.width(),
          middle.height(),
          left + cut_left,
          top + cut_top,
        );
      }
      completed += 1;
      progress(completed as f32 / tile_count as f32);
    }
  }

  if completed == 0 {
    return Ok(None);
  }
  let refined = Mask::from_values(width, height, alpha)
    .resized(part_width, part_height, TransformAlgorithm::Bilinear)
    .placed(photo_width, photo_height, (left, top));
  Ok(Some(refined))
}

fn tile_starts(length: u32, tile_size: u32, overlap: u32) -> Vec<u32> {
  if length <= tile_size {
    return vec![0];
  }
  let step = tile_size - overlap;
  let count = (length - overlap).div_ceil(step);
  (0..count)
    .map(|index| if index + 1 == count { length - tile_size } else { index * step })
    .collect()
}

fn paste_max(target: &mut [u8], target_width: u32, source: &[u8], source_width: u32, source_height: u32, x: u32, y: u32) {
  for row in 0..source_height as usize {
    let target_start = (y as usize + row) * target_width as usize + x as usize;
    let source_start = row * source_width as usize;
    for (destination, value) in target[target_start..target_start + source_width as usize]
      .iter_mut()
      .zip(&source[source_start..source_start + source_width as usize])
    {
      *destination = (*destination).max(*value);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_side_that_fits_in_one_tile_is_one_tile() {
    assert_eq!(tile_starts(1, 1024, 128), vec![0]);
    assert_eq!(tile_starts(1024, 1024, 128), vec![0]);
  }

  #[test]
  fn tiles_start_at_the_edge_end_at_the_other_and_overlap_by_at_least_the_overlap() {
    for (tile, overlap) in [(1024, 128), (256, 0), (64, 32), (1024, 1023)] {
      for length in (1..4000).chain([tile, tile + 1, tile * 2, tile * 2 - overlap, tile * 3 + 7]) {
        let starts = tile_starts(length, tile, overlap);
        let context = format!("length {length}, tile {tile}, overlap {overlap}: {starts:?}");
        assert_eq!(starts[0], 0, "{context}");
        if length > tile {
          assert_eq!(*starts.last().unwrap() + tile, length, "the last tile ends at the side: {context}");
        }
        for pair in starts.windows(2) {
          assert!(pair[1] > pair[0], "tiles move forward: {context}");
          assert!(pair[1] - pair[0] <= tile - overlap, "neighbors share at least the overlap, so no pixel is left out: {context}");
        }
      }
    }
  }

  #[test]
  fn pasting_keeps_the_larger_value_and_leaves_everything_else_alone() {
    let mut target = vec![5u8; 4 * 3];
    // A 2 x 2 piece at (1, 1): its first row is smaller than what is there, its second is larger.
    paste_max(&mut target, 4, &[1, 9, 7, 2], 2, 2, 1, 1);
    assert_eq!(target, vec![5, 5, 5, 5, 5, 5, 9, 5, 5, 7, 5, 5]);
  }

  #[test]
  fn a_piece_that_ends_at_the_edge_of_the_target_fits() {
    let mut target = vec![0u8; 3 * 2];
    paste_max(&mut target, 3, &[1, 2, 3, 4], 2, 2, 1, 0);
    assert_eq!(target, vec![0, 1, 2, 0, 3, 4]);
  }

  #[test]
  fn the_settings_are_kept_as_given() {
    let options = EdgeMatteOptions::new().with_unknown_band(0.02).with_scale(2.0).with_tile_size(512).with_overlap(64);
    assert_eq!((options.unknown_band, options.scale, options.tile_size, options.overlap), (0.02, 2.0, 512, 64));
    let defaults = EdgeMatteOptions::default();
    assert_eq!((defaults.tile_size, defaults.overlap), (1024, 128));
  }
}
