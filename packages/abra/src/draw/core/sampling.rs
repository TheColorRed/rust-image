//! Supersampling grid for anti-aliasing.

/// Defines a supersampling grid for anti-aliasing.
pub(crate) struct SampleGrid {
  /// Number of samples per pixel side (e.g., 4 means 4x4 = 16 samples).
  pub side_samples: u32,
}

impl SampleGrid {
  /// Creates a new sample grid from an anti-aliasing level.
  /// Level is clamped between 1 and 16.
  pub fn from_aa_level(p_level: u32) -> Self {
    SampleGrid {
      side_samples: p_level.max(1).min(16),
    }
  }

  /// Returns total number of samples per pixel.
  pub fn total_samples(&self) -> u32 {
    self.side_samples * self.side_samples
  }

  /// Returns the inverse of side_samples for efficient division.
  pub fn inv_side(&self) -> f32 {
    1.0 / self.side_samples as f32
  }

  /// Iterates over all sub-pixel sample positions for a given pixel.
  /// Yields (sub_x, sub_y) coordinates centered within each sample.
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
