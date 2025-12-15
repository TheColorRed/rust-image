//! Supersampling grid used by the rasterizer to reduce aliasing.
//!
//! `SampleGrid` represents a regular NxN sampling pattern inside each
//! pixel that the `Rasterizer` can use to evaluate shader coverage and
//! color. A higher number of samples increases visual quality by
//! reducing aliasing at edges at the expense of CPU work (samples * per-
//! pixel cost).
//!
//! Key points:
//! - Coordinates for returned sample positions are in device space (not
//!   normalized), and are centered within each subpixel cell.
//! - `side_samples` controls how many samples per pixel side are used,
//!   so `total_samples = side_samples * side_samples`.
//! - Typical choices: `1` (no supersampling), `2` (2x2, cheap), `4` or
//!   `8` for progressively higher quality. `SampleGrid` will clamp the
//!   `side_samples` level between 1 and 16 for safety/performance.
//!
//! Example
//! ```ignore
//! let grid = SampleGrid::from_aa_level(4); // 4x4, 16 samples per pixel
//! for (sx, sy) in grid.samples(10, 20) {
//!   // Evaluate shader at (sx, sy)
//! }
//! ```

/// Defines a supersampling grid for anti-aliasing.
///
/// A `SampleGrid` represents a regular NxN subpixel pattern used when
/// computing anti-aliased rasterization results. Each pixel is subdivided
/// into `side_samples * side_samples` samples and the shader is evaluated
/// at each sample position. The final color is averaged across samples
/// that fall within the coverage mask.
///
/// Example
/// ```ignore
/// let grid = SampleGrid::from_aa_level(4); // 4x4 grid => 16 total samples
/// assert_eq!(grid.total_samples(), 16);
/// ```
pub struct SampleGrid {
  /// Number of samples per pixel side (e.g., 4 means 4x4 = 16 samples).
  pub side_samples: u32,
}

impl SampleGrid {
  /// Creates a new sample grid from an anti-aliasing level.
  ///
  /// `p_level` is the per-side sample count, clamped between `1` and
  /// `16` to avoid pathological memory/perf and maintain reasonable
  /// quality. A value of `1` disables supersampling (1 sample per pixel).
  ///
  /// Example
  /// ```ignore
  /// let grid = SampleGrid::from_aa_level(2); // 2x2 samples
  /// ```
  pub fn from_aa_level(p_level: u32) -> Self {
    SampleGrid {
      side_samples: p_level.clamp(1, 16),
    }
  }

  /// Returns the total number of subpixel samples (side_samples * side_samples).
  pub fn total_samples(&self) -> u32 {
    self.side_samples * self.side_samples
  }

  /// Returns the inverse of the side sample count for use in sample positioning.
  pub fn inv_side(&self) -> f32 {
    1.0 / self.side_samples as f32
  }

  /// Iterates over all subpixel sample positions for the pixel at `(p_x, p_y)`.
  ///
  /// The returned iterator yields the coordinates of each sample point in
  /// device space. Coordinates are centered in each subpixel; e.g., for a
  /// 2x2 grid the sample offsets are at (0.25, 0.25), (0.75, 0.25), etc.
  ///
  /// Example
  /// ```ignore
  /// for (sx, sy) in SampleGrid::from_aa_level(2).samples(10, 20) {
  ///   // sx, sy hold precise sample coordinates (f32) inside the pixel
  /// }
  /// ```
  pub fn samples(&self, p_x: u32, p_y: u32) -> impl Iterator<Item = (f32, f32)> + '_ {
    let x = p_x as f32;
    let y = p_y as f32;
    let inv = self.inv_side();
    let side = self.side_samples;

    (0..side).flat_map(move |sx| {
      (0..side).map(move |sy| {
        let sub_x = x + (sx as f32 + 0.5) * inv;
        let sub_y = y + (sy as f32 + 0.5) * inv;
        (sub_x, sub_y)
      })
    })
  }
}
