use primitives::Image;

use rayon::prelude::*;

/// Represents the color histogram of an image.
#[derive(Debug, Clone)]
pub struct Histogram {
  /// Red channel histogram (heap-allocated to keep struct small on stack).
  red: Box<[u64; 256]>,
  /// Green channel histogram.
  green: Box<[u64; 256]>,
  /// Blue channel histogram.
  blue: Box<[u64; 256]>,
  /// Alpha channel histogram.
  alpha: Box<[u64; 256]>,
}

impl Histogram {
  /// Creates a new, empty histogram.
  pub fn new() -> Self {
    Self {
      red: Box::new([0; 256]),
      green: Box::new([0; 256]),
      blue: Box::new([0; 256]),
      alpha: Box::new([0; 256]),
    }
  }
  /// Computes the histogram from the given image.
  /// - `p_image`: The image to compute the histogram from.
  /// Returns the computed histogram.
  pub fn from_image(p_image: &Image) -> Self {
    let src = p_image.rgba();
    Self::from_rgba(src)
  }
  pub fn from_image_skip_transparent(p_image: &Image) -> Self {
    let src = p_image.rgba();
    Self::from_rgba_skip_transparent(src)
  }
  /// Computes the histogram from the given RGBA pixel data.
  /// - `rgba`: The RGBA pixel data.
  /// Returns the computed histogram.
  pub fn from_rgba(rgba: &[u8]) -> Self {
    rgba
      .par_chunks(4)
      .fold(
        || Histogram::new(),
        |mut acc, px| {
          acc.red[px[0] as usize] += 1;
          acc.green[px[1] as usize] += 1;
          acc.blue[px[2] as usize] += 1;
          acc.alpha[px[3] as usize] += 1;
          acc
        },
      )
      .reduce(
        || Histogram::new(),
        |mut a, b| {
          for i in 0..256 {
            a.red[i] += b.red[i];
            a.green[i] += b.green[i];
            a.blue[i] += b.blue[i];
            a.alpha[i] += b.alpha[i];
          }
          a
        },
      )
  }
  /// Computes the histogram from the given RGBA pixel data, skipping fully-transparent pixels.
  /// - `rgba`: The RGBA pixel data.
  /// Returns the computed histogram.
  pub fn from_rgba_skip_transparent(rgba: &[u8]) -> Self {
    rgba
      .par_chunks(4)
      .fold(
        || Histogram::new(),
        |mut acc, px| {
          let a = px[3];
          if a == 0 {
            return acc;
          }
          acc.red[px[0] as usize] += 1;
          acc.green[px[1] as usize] += 1;
          acc.blue[px[2] as usize] += 1;
          acc.alpha[px[3] as usize] += 1;
          acc
        },
      )
      .reduce(
        || Histogram::new(),
        |mut a, b| {
          for i in 0..256 {
            a.red[i] += b.red[i];
            a.green[i] += b.green[i];
            a.blue[i] += b.blue[i];
            a.alpha[i] += b.alpha[i];
          }
          a
        },
      )
  }
  /// Returns a reference to the red channel histogram.
  pub fn red(&self) -> &[u64; 256] {
    &*self.red
  }
  /// Returns a reference to the green channel histogram.
  pub fn green(&self) -> &[u64; 256] {
    &*self.green
  }
  /// Returns a reference to the blue channel histogram.
  pub fn blue(&self) -> &[u64; 256] {
    &*self.blue
  }
  /// Returns a reference to the alpha channel histogram.
  pub fn alpha(&self) -> &[u64; 256] {
    &*self.alpha
  }
  /// Returns the total number of pixels counted in the histogram.
  pub fn total_pixels(&self) -> u64 {
    self.red.iter().sum::<u64>()
  }
  /// Compute low/high clip bounds for a given channel histogram slice using
  /// a symmetric clip fraction. Returns (low, high) in the 0..=255 range.
  ///
  /// Behavior mirrors the closure previously embedded in `auto_color`:
  /// - clip_fraction is applied symmetrically to both tails.
  /// - if the histogram is empty, returns (0, 255) as identity mapping.
  /// - if the histogram collapses to a single non-zero bin returns (bin, bin).
  pub fn clip_bounds_from_slice(&self, hist: &[u64; 256], clip_fraction: f32) -> (u8, u8) {
    let total_pixels = hist.iter().sum::<u64>();
    let mut lo_bin = 0usize;
    let mut hi_bin = 255usize;
    let clip_count = (clip_fraction * (total_pixels as f32)).round() as u64;
    let mut cum: u64 = 0;
    for (i, &c) in hist.iter().enumerate() {
      cum += c;
      if cum >= clip_count {
        lo_bin = i;
        break;
      }
    }
    cum = 0;
    for (i, &c) in hist.iter().enumerate().rev() {
      cum += c;
      if cum >= clip_count {
        hi_bin = i;
        break;
      }
    }
    if lo_bin >= hi_bin {
      // Fallback: use min/max non-zero bins or defaults 0/255
      let mut min = 0usize;
      let mut max = 255usize;
      for (i, &c) in hist.iter().enumerate() {
        if c > 0 {
          min = i;
          break;
        }
      }
      for (i, &c) in hist.iter().enumerate().rev() {
        if c > 0 {
          max = i;
          break;
        }
      }
      lo_bin = min;
      hi_bin = max;
    }
    (lo_bin as u8, hi_bin as u8)
  }
  /// Gets the red channel clip bounds for the given clip fraction.
  /// - `clip_fraction`: The fraction of pixels to clip from each tail.
  /// Returns a tuple containing the low and high clip bounds.
  pub fn red_clip_bounds(&self, clip_fraction: f32) -> (u8, u8) {
    self.clip_bounds_from_slice(&*self.red, clip_fraction)
  }
  /// Gets the green channel clip bounds for the given clip fraction.
  /// - `clip_fraction`: The fraction of pixels to clip from each tail.
  /// Returns a tuple containing the low and high clip bounds.
  pub fn green_clip_bounds(&self, clip_fraction: f32) -> (u8, u8) {
    self.clip_bounds_from_slice(&*self.green, clip_fraction)
  }
  /// Gets the blue channel clip bounds for the given clip fraction.
  /// - `clip_fraction`: The fraction of pixels to clip from each tail.
  /// Returns a tuple containing the low and high clip bounds.
  pub fn blue_clip_bounds(&self, clip_fraction: f32) -> (u8, u8) {
    self.clip_bounds_from_slice(&*self.blue, clip_fraction)
  }
  /// Construct a lookup table (LUT) for levels mapping given clip_fraction.
  /// The LUT maps an 8-bit channel value [0..255] to an 8-bit mapped value [0..255]
  /// according to the lo/hi clip bounds computed for the histogram slice.
  pub fn levels_lut_from_slice(&self, hist: &[u64; 256], clip_fraction: f32) -> [u8; 256] {
    let (lo, hi) = self.clip_bounds_from_slice(hist, clip_fraction);
    let lo_i = lo as i32;
    let hi_i = hi as i32;
    let denom = ((hi_i - lo_i) as f32).max(1.0);
    let mut lut: [u8; 256] = [0; 256];
    for i in 0..256 {
      let val = if i <= lo_i {
        0.0
      } else if i >= hi_i {
        1.0
      } else {
        ((i - lo_i) as f32) / denom
      };
      lut[i as usize] = (val * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    lut
  }
  /// Gets the red channel levels lookup table (LUT) for the given clip fraction.
  /// - `clip_fraction`: The fraction of pixels to clip from each tail.
  /// Returns the red channel levels LUT.
  pub fn red_levels_lut(&self, clip_fraction: f32) -> [u8; 256] {
    self.levels_lut_from_slice(&*self.red, clip_fraction)
  }
  /// Gets the green channel levels lookup table (LUT) for the given clip fraction.
  /// - `clip_fraction`: The fraction of pixels to clip from each tail.
  /// Returns the green channel levels LUT.
  pub fn green_levels_lut(&self, clip_fraction: f32) -> [u8; 256] {
    self.levels_lut_from_slice(&*self.green, clip_fraction)
  }
  /// Gets the blue channel levels lookup table (LUT) for the given clip fraction.
  /// - `clip_fraction`: The fraction of pixels to clip from each tail.
  /// Returns the blue channel levels LUT.
  pub fn blue_levels_lut(&self, clip_fraction: f32) -> [u8; 256] {
    self.levels_lut_from_slice(&*self.blue, clip_fraction)
  }

  /// Clears all histogram data, resetting all channels to zero.
  pub fn clear(&mut self) {
    self.red.fill(0);
    self.green.fill(0);
    self.blue.fill(0);
    self.alpha.fill(0);
  }

  /// Returns mutable references to the R, G, B histogram arrays for direct manipulation.
  /// - Returns: Tuple of mutable references to (red, green, blue) histograms.
  pub fn rgb_mut(&mut self) -> (&mut [u64; 256], &mut [u64; 256], &mut [u64; 256]) {
    (&mut *self.red, &mut *self.green, &mut *self.blue)
  }

  /// Fast histogram-based median finder for a single channel.
  /// Uses counting sort approach: O(256) instead of O(n log n).
  /// - `hist`: The histogram array to find the median from.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the median value (0-255).
  #[inline]
  pub fn median_from_hist(hist: &[u64; 256], count: u64) -> u8 {
    let half = count / 2;
    let mut cumulative = 0u64;
    for (val, &freq) in hist.iter().enumerate() {
      cumulative += freq;
      if cumulative > half {
        return val as u8;
      }
    }
    255 // fallback
  }

  /// Calculate the median RGB values from the current histogram data.
  /// - Returns: Tuple of (red_median, green_median, blue_median).
  pub fn rgb_medians(&self) -> (u8, u8, u8) {
    let total = self.total_pixels();
    (
      Self::median_from_hist(&*self.red, total),
      Self::median_from_hist(&*self.green, total),
      Self::median_from_hist(&*self.blue, total),
    )
  }

  /// Gets the median value from the red channel histogram.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the median red value (0-255).
  #[inline]
  pub fn red_median(&self, count: u64) -> u8 {
    Self::median_from_hist(&*self.red, count)
  }

  /// Gets the median value from the green channel histogram.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the median green value (0-255).
  #[inline]
  pub fn green_median(&self, count: u64) -> u8 {
    Self::median_from_hist(&*self.green, count)
  }

  /// Gets the median value from the blue channel histogram.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the median blue value (0-255).
  #[inline]
  pub fn blue_median(&self, count: u64) -> u8 {
    Self::median_from_hist(&*self.blue, count)
  }

  /// Computes the mean (average) value from a histogram channel.
  /// - `hist`: The histogram array.
  /// - `count`: The total number of samples.
  /// Returns the mean value (0-255).
  #[inline]
  pub fn mean_from_hist(hist: &[u64; 256], count: u64) -> u8 {
    if count == 0 {
      return 0;
    }
    let mut sum = 0u64;
    for (val, &freq) in hist.iter().enumerate() {
      sum += (val as u64) * freq;
    }
    (sum / count).min(255) as u8
  }

  /// Gets the mean value from the red channel histogram.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the mean red value (0-255).
  #[inline]
  pub fn red_mean(&self, count: u64) -> u8 {
    Self::mean_from_hist(&*self.red, count)
  }

  /// Gets the mean value from the green channel histogram.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the mean green value (0-255).
  #[inline]
  pub fn green_mean(&self, count: u64) -> u8 {
    Self::mean_from_hist(&*self.green, count)
  }

  /// Gets the mean value from the blue channel histogram.
  /// - `count`: The total number of samples in the histogram.
  /// Returns the mean blue value (0-255).
  #[inline]
  pub fn blue_mean(&self, count: u64) -> u8 {
    Self::mean_from_hist(&*self.blue, count)
  }

  /// Computes a weighted average from a histogram, only considering values within a threshold range.
  /// This is useful for bilateral/edge-aware filtering.
  /// - `hist`: The histogram array.
  /// - `center_value`: The center pixel value to compare against.
  /// - `threshold`: Maximum difference allowed from center value.
  /// Returns the weighted average value (0-255), or center_value if no pixels match.
  #[inline]
  pub fn weighted_average_in_range(hist: &[u64; 256], center_value: u8, threshold: u8) -> u8 {
    let cv = center_value as i32;
    let thr = threshold as i32;
    let min_val = (cv - thr).max(0) as usize;
    let max_val = (cv + thr).min(255) as usize;

    let mut sum = 0u64;
    let mut count = 0u64;

    for val in min_val..=max_val {
      let freq = hist[val];
      if freq > 0 {
        sum += (val as u64) * freq;
        count += freq;
      }
    }

    if count == 0 {
      center_value
    } else {
      (sum / count).min(255) as u8
    }
  }

  /// Gets the weighted average from the red channel within threshold range.
  /// - `center_value`: The center pixel value.
  /// - `threshold`: Maximum difference allowed.
  /// Returns the weighted average red value (0-255).
  #[inline]
  pub fn red_weighted_average(&self, center_value: u8, threshold: u8) -> u8 {
    Self::weighted_average_in_range(&*self.red, center_value, threshold)
  }

  /// Gets the weighted average from the green channel within threshold range.
  /// - `center_value`: The center pixel value.
  /// - `threshold`: Maximum difference allowed.
  /// Returns the weighted average green value (0-255).
  #[inline]
  pub fn green_weighted_average(&self, center_value: u8, threshold: u8) -> u8 {
    Self::weighted_average_in_range(&*self.green, center_value, threshold)
  }

  /// Gets the weighted average from the blue channel within threshold range.
  /// - `center_value`: The center pixel value.
  /// - `threshold`: Maximum difference allowed.
  /// Returns the weighted average blue value (0-255).
  #[inline]
  pub fn blue_weighted_average(&self, center_value: u8, threshold: u8) -> u8 {
    Self::weighted_average_in_range(&*self.blue, center_value, threshold)
  }

  /// Finds the value at a given percentile in the histogram.
  /// - `hist`: The histogram array.
  /// - `count`: Total number of samples.
  /// - `percentile`: Value between 0.0 and 1.0 (e.g., 0.5 for median).
  /// Returns the value at the specified percentile (0-255).
  #[inline]
  pub fn percentile_from_hist(hist: &[u64; 256], count: u64, percentile: f32) -> u8 {
    let target = (count as f32 * percentile.clamp(0.0, 1.0)) as u64;
    let mut cumulative = 0u64;
    for (val, &freq) in hist.iter().enumerate() {
      cumulative += freq;
      if cumulative >= target {
        return val as u8;
      }
    }
    255
  }

  /// Gets the value at a percentile from the red channel.
  /// - `count`: Total number of samples.
  /// - `percentile`: Value between 0.0 and 1.0.
  /// Returns the red value at the percentile (0-255).
  #[inline]
  pub fn red_percentile(&self, count: u64, percentile: f32) -> u8 {
    Self::percentile_from_hist(&*self.red, count, percentile)
  }

  /// Gets the value at a percentile from the green channel.
  /// - `count`: Total number of samples.
  /// - `percentile`: Value between 0.0 and 1.0.
  /// Returns the green value at the percentile (0-255).
  #[inline]
  pub fn green_percentile(&self, count: u64, percentile: f32) -> u8 {
    Self::percentile_from_hist(&*self.green, count, percentile)
  }

  /// Gets the value at a percentile from the blue channel.
  /// - `count`: Total number of samples.
  /// - `percentile`: Value between 0.0 and 1.0.
  /// Returns the blue value at the percentile (0-255).
  #[inline]
  pub fn blue_percentile(&self, count: u64, percentile: f32) -> u8 {
    Self::percentile_from_hist(&*self.blue, count, percentile)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn histogram_skip_transparent_counts() {
    // build a small rgba buffer with 6 pixels, 2 with alpha=0
    let rgba: Vec<u8> = vec![
      10, 20, 30, 255, // opaque
      10, 20, 30, 0, // transparent -> should be skipped
      255, 0, 0, 255, // opaque red
      0, 255, 0, 255, // opaque green
      0, 0, 255, 255, // opaque blue
      10, 20, 30, 255, // opaque duplicate
    ];
    let hist = Histogram::from_rgba_skip_transparent(&rgba);
    // There are 5 opaque pixels, so red histogram sum should be 5
    assert_eq!(hist.total_pixels(), 5);
    // Check a couple of channel counts
    assert_eq!(hist.red()[10], 2);
    assert_eq!(hist.red()[255], 1);
    assert_eq!(hist.green()[255], 1);
    assert_eq!(hist.blue()[255], 1);
    // Transparent pixel should be skipped by the "skip_transparent" constructor
    assert_eq!(hist.alpha()[0], 0);
  }

  #[test]
  fn clip_bounds_zero_clip() {
    let mut hist = Histogram::new();
    // Set some counts
    hist.red[10] = 2;
    hist.red[200] = 3;
    // clip fraction zero should return identity 0..255
    assert_eq!(hist.red_clip_bounds(0.0), (0u8, 255u8));
  }

  #[test]
  fn clip_bounds_uniform_bin_returns_same_bin() {
    let mut hist = Histogram::new();
    // All counts in one bin -> fallback should return the same bin
    hist.red[50] = 42;
    // Use a clip fraction large enough to ensure clip_count > 0
    assert_eq!(hist.red_clip_bounds(0.05), (50u8, 50u8));
  }

  #[test]
  fn clip_bounds_empty_hist_returns_identity() {
    let hist = Histogram::new();
    assert_eq!(hist.red_clip_bounds(0.01), (0u8, 255u8));
  }

  #[test]
  fn clip_bounds_known_distribution() {
    let mut hist = Histogram::new();
    // construct known red distribution: 5 at 10, 30 at 200, 5 at 240
    hist.red[10] = 5;
    hist.red[200] = 30;
    hist.red[240] = 5;
    // total 40, clip_fraction 0.125 => clip_count = 5
    assert_eq!(hist.red_clip_bounds(0.125), (10u8, 240u8));
  }

  #[test]
  fn levels_lut_identity_with_zero_clip() {
    let mut hist = Histogram::new();
    // Any distribution doesn't matter for clip_fraction == 0
    hist.red[10] = 10;
    let lut = hist.red_levels_lut(0.0);
    // lut[i] should equal i for identity mapping
    assert_eq!(lut[0], 0u8);
    assert_eq!(lut[128], 128u8);
    assert_eq!(lut[255], 255u8);
  }

  #[test]
  fn levels_lut_known_bounds() {
    let mut hist = Histogram::new();
    hist.red[10] = 5;
    hist.red[200] = 30;
    hist.red[240] = 5;
    let lut = hist.red_levels_lut(0.125);
    // Boundaries
    assert_eq!(lut[10], 0u8);
    assert_eq!(lut[240], 255u8);
    // mid mapping check for 125 -> approx 128
    assert!((lut[125] as i32 - 128).abs() <= 1, "lut[125]={} expected ~128", lut[125]);
  }
}
