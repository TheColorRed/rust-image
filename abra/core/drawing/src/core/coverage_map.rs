//! Anti-aliased rasters of coverage masks.
//!
//! A [`CoverageMap`] records how much of each pixel in a window a [`CoverageMask`] covers, from 0 to 1. It is the
//! bridge between a shape and a per-pixel operation: rasterize once, then read coverage as an alpha value, find
//! which pixels are inside, or measure how far each pixel is from the shape's edge.

use rayon::prelude::*;

use crate::{CoverageMask, SampleGrid};

/// How much of each pixel in a rectangular window of an image a shape covers, from 0 to 1.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageMap {
  x: i32,
  y: i32,
  width: usize,
  height: usize,
  values: Vec<f32>,
}

impl CoverageMap {
  /// Rasterizes `p_mask` over the window of pixels whose top-left pixel is `(p_x, p_y)`, testing a grid of points
  /// in each pixel. Pixels outside the mask's bounds are skipped, so a small shape in a large window is cheap.
  /// - `p_mask`: The shape.
  /// - `p_x`, `p_y`: The top-left pixel of the window, in image pixels.
  /// - `p_width`, `p_height`: The size of the window in pixels.
  /// - `p_grid`: How many points are tested in each pixel. More gives smoother edges and costs more.
  pub fn rasterize(
    p_mask: &dyn CoverageMask, p_x: i32, p_y: i32, p_width: usize, p_height: usize, p_grid: SampleGrid,
  ) -> CoverageMap {
    let (min_x, min_y, max_x, max_y) = p_mask.bounds().unwrap_or((f32::MIN, f32::MIN, f32::MAX, f32::MAX));
    let total = p_grid.total_samples() as f32;
    let mut values = vec![0.0; p_width * p_height];
    if p_width > 0 {
      values.par_chunks_mut(p_width).enumerate().for_each(|(row, line)| {
        let y = p_y + row as i32;
        // Most of a long or thin shape's window is empty, and testing a point is not free.
        if ((y + 1) as f32) < min_y || (y as f32) > max_y {
          return;
        }
        for (column, value) in line.iter_mut().enumerate() {
          let x = p_x + column as i32;
          if ((x + 1) as f32) < min_x || (x as f32) > max_x {
            continue;
          }
          // Pixel (x, y) spans x..x+1; the grid places its points in the middle of each cell.
          let hits = sample_points(&p_grid, x, y).filter(|&(sx, sy)| p_mask.contains(sx, sy)).count();
          *value = hits as f32 / total;
        }
      });
    }
    CoverageMap {
      x: p_x,
      y: p_y,
      width: p_width,
      height: p_height,
      values,
    }
  }

  /// The left edge of the window, in image pixels.
  pub fn x(&self) -> i32 {
    self.x
  }

  /// The top edge of the window, in image pixels.
  pub fn y(&self) -> i32 {
    self.y
  }

  /// The width of the window in pixels.
  pub fn width(&self) -> usize {
    self.width
  }

  /// The height of the window in pixels.
  pub fn height(&self) -> usize {
    self.height
  }

  /// The coverage of every pixel of the window, row by row.
  pub fn values(&self) -> &[f32] {
    &self.values
  }

  /// Whether the shape covers no pixel of the window at all.
  pub fn is_empty(&self) -> bool {
    self.values.iter().all(|&value| value <= 0.0)
  }

  /// Which pixels of the window the shape touches at all, row by row.
  pub fn covered(&self) -> Vec<bool> {
    self.values.iter().map(|&value| value > 0.0).collect()
  }

  /// For every pixel of the window, how far it is from the nearest covered pixel: 0 on the shape, 1 next to it, and
  /// so on outwards. Distances are approximate, following steps along rows, columns and diagonals.
  pub fn distance_outside(&self) -> Vec<f32> {
    distance_field(&self.covered(), self.width, self.height, true)
  }

  /// For every pixel of the window, how far it is from the nearest pixel the shape does not touch: 0 off the shape,
  /// 1 on its edge, and so on inwards. Distances are approximate, following steps along rows, columns and diagonals.
  pub fn distance_inside(&self) -> Vec<f32> {
    distance_field(&self.covered(), self.width, self.height, false)
  }
}

/// The sample positions of the grid for the pixel `(p_x, p_y)`, which may be negative.
fn sample_points(p_grid: &SampleGrid, p_x: i32, p_y: i32) -> impl Iterator<Item = (f32, f32)> + '_ {
  let (side, inv) = (p_grid.side_samples, p_grid.inv_side());
  (0..side).flat_map(move |sy| {
    (0..side).map(move |sx| (p_x as f32 + (sx as f32 + 0.5) * inv, p_y as f32 + (sy as f32 + 0.5) * inv))
  })
}

/// For every pixel, the distance to the nearest pixel of a set, approximated with a two-pass sweep.
/// - `p_set`: Marks which pixels are in the set.
/// - `p_to_set`: Measure to the pixels in the set when `true`, or to the pixels outside it when `false`.
fn distance_field(p_set: &[bool], p_width: usize, p_height: usize, p_to_set: bool) -> Vec<f32> {
  const DIAGONAL: f32 = std::f32::consts::SQRT_2;
  let mut distance: Vec<f32> =
    p_set.iter().map(|&in_set| if in_set == p_to_set { 0.0 } else { f32::MAX / 4.0 }).collect();

  let mut relax = |x: usize, y: usize, dx: i32, dy: i32, cost: f32| {
    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
    if nx >= 0 && ny >= 0 && (nx as usize) < p_width && (ny as usize) < p_height {
      let candidate = distance[ny as usize * p_width + nx as usize] + cost;
      let slot = &mut distance[y * p_width + x];
      if candidate < *slot {
        *slot = candidate;
      }
    }
  };

  for y in 0..p_height {
    for x in 0..p_width {
      relax(x, y, -1, 0, 1.0);
      relax(x, y, -1, -1, DIAGONAL);
      relax(x, y, 0, -1, 1.0);
      relax(x, y, 1, -1, DIAGONAL);
    }
  }
  for y in (0..p_height).rev() {
    for x in (0..p_width).rev() {
      relax(x, y, 1, 0, 1.0);
      relax(x, y, 1, 1, DIAGONAL);
      relax(x, y, 0, 1, 1.0);
      relax(x, y, -1, 1, DIAGONAL);
    }
  }
  distance
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::RectCoverage;

  #[test]
  fn coverage_is_partial_along_edges_that_cut_through_pixels() {
    // Covers x 1.5..4 and y 1..3, so column 1 is half covered.
    let map =
      CoverageMap::rasterize(&RectCoverage::new((1.5, 1.0), (2.5, 2.0)), 0, 0, 6, 4, SampleGrid::from_aa_level(4));
    let at = |x: usize, y: usize| map.values()[y * map.width() + x];
    assert_eq!(at(2, 1), 1.0);
    assert_eq!(at(1, 1), 0.5);
    assert_eq!(at(0, 0), 0.0);
    assert!(!map.is_empty());
  }

  #[test]
  fn the_window_can_start_anywhere() {
    let map = CoverageMap::rasterize(&RectCoverage::new((-3, -3), (2, 2)), -4, -4, 4, 4, SampleGrid::from_aa_level(2));
    assert_eq!((map.x(), map.y(), map.width(), map.height()), (-4, -4, 4, 4));
    assert_eq!(map.values()[1 * 4 + 1], 1.0);
    let missed = CoverageMap::rasterize(&RectCoverage::new((50, 50), (2, 2)), 0, 0, 4, 4, SampleGrid::from_aa_level(2));
    assert!(missed.is_empty());
  }

  #[test]
  fn distances_are_measured_from_the_edge() {
    // A 3x3 block in the middle of a 7x7 window.
    let map = CoverageMap::rasterize(&RectCoverage::new((2, 2), (3, 3)), 0, 0, 7, 7, SampleGrid::from_aa_level(2));
    let (outside, inside) = (map.distance_outside(), map.distance_inside());
    assert_eq!(outside[3 * 7 + 3], 0.0);
    assert_eq!(outside[3 * 7 + 1], 1.0);
    assert_eq!(outside[3 * 7 + 0], 2.0);
    assert_eq!(inside[3 * 7 + 2], 1.0);
    assert_eq!(inside[3 * 7 + 3], 2.0);
    assert_eq!(inside[0], 0.0);
  }
}
