//! Value histograms for image channels.
//!
//! [`Bins`] counts how often each 0-255 value occurs in one channel and answers statistics about it (median,
//! percentile, mean, mode, clipping bounds). [`Histogram`] holds one [`Bins`] per RGBA channel of an image.

use rayon::prelude::*;

use crate::channels::Channel;

/// Counts of each 0-255 value in one channel, with a running total.
///
/// Filters that slide a window across an image can [`Bins::add`] and [`Bins::remove`] values as the window moves;
/// the statistics read the running total, so they never need to re-count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bins {
  counts: [u64; 256],
  total: u64,
}

impl Default for Bins {
  fn default() -> Self {
    Bins {
      counts: [0; 256],
      total: 0,
    }
  }
}

impl Bins {
  /// Creates empty bins.
  pub fn new() -> Self {
    Self::default()
  }

  /// Counts one occurrence of `p_value`.
  #[inline]
  pub fn add(&mut self, p_value: u8) {
    self.counts[p_value as usize] += 1;
    self.total += 1;
  }

  /// Removes one occurrence of `p_value`. Does nothing when the value has not been counted.
  #[inline]
  pub fn remove(&mut self, p_value: u8) {
    let count = &mut self.counts[p_value as usize];
    if *count > 0 {
      *count -= 1;
      self.total -= 1;
    }
  }

  /// Adds every count of `p_other` to these bins.
  pub fn merge(&mut self, p_other: &Bins) {
    for (count, other) in self.counts.iter_mut().zip(p_other.counts.iter()) {
      *count += other;
    }
    self.total += p_other.total;
  }

  /// Removes every count.
  pub fn clear(&mut self) {
    self.counts.fill(0);
    self.total = 0;
  }

  /// How many times each value was counted.
  pub fn counts(&self) -> &[u64; 256] {
    &self.counts
  }

  /// How many values were counted.
  pub fn total(&self) -> u64 {
    self.total
  }

  /// The value at the given fraction of the sorted values: `0.0` is the smallest, `0.5` the median, and `1.0` the
  /// largest. Empty bins return `0`.
  /// - `p_fraction`: The position in the sorted values, clamped to `0.0..=1.0`.
  pub fn percentile(&self, p_fraction: f32) -> u8 {
    if self.total == 0 {
      return 0;
    }
    let index = ((self.total as f64 * p_fraction.clamp(0.0, 1.0) as f64) as u64).min(self.total - 1);
    let mut seen = 0u64;
    for (value, &count) in self.counts.iter().enumerate() {
      seen += count;
      if seen > index {
        return value as u8;
      }
    }
    255
  }

  /// The median value (the lower median for an even count).
  pub fn median(&self) -> u8 {
    self.percentile(0.5)
  }

  /// The mean value, rounded down. Empty bins return `0`.
  pub fn mean(&self) -> u8 {
    if self.total == 0 {
      return 0;
    }
    let sum: u64 = self.counts.iter().enumerate().map(|(value, &count)| value as u64 * count).sum();
    (sum / self.total).min(255) as u8
  }

  /// The most frequent value. Ties go to the lowest value.
  pub fn mode(&self) -> u8 {
    // Iterating in reverse makes `max_by_key`, which keeps the last maximum, keep the lowest value.
    self.counts.iter().enumerate().rev().max_by_key(|(_, count)| **count).map_or(0, |(value, _)| value as u8)
  }

  /// The mean of the counted values within `p_threshold` of `p_center`, or `p_center` when there are none.
  pub fn weighted_average(&self, p_center: u8, p_threshold: u8) -> u8 {
    let min = p_center.saturating_sub(p_threshold) as usize;
    let max = p_center.saturating_add(p_threshold) as usize;
    let (sum, count) = self.counts[min..=max]
      .iter()
      .enumerate()
      .fold((0u64, 0u64), |(sum, count), (offset, &n)| (sum + (min + offset) as u64 * n, count + n));
    if count == 0 { p_center } else { (sum / count).min(255) as u8 }
  }

  /// The darkest and brightest values after ignoring `p_clip_fraction` of the values at each end, as used by
  /// auto-levels. When that leaves nothing, the smallest and largest counted values are used.
  pub fn clip_bounds(&self, p_clip_fraction: f32) -> (u8, u8) {
    let clip_count = (p_clip_fraction * self.total as f32).round() as u64;
    let low = first_reaching(&self.counts, 0..256, clip_count).unwrap_or(0);
    let high = first_reaching(&self.counts, (0..256).rev(), clip_count).unwrap_or(255);
    if low < high {
      return (low as u8, high as u8);
    }
    let low = self.counts.iter().position(|&count| count > 0).unwrap_or(0);
    let high = self.counts.iter().rposition(|&count| count > 0).unwrap_or(255);
    (low as u8, high as u8)
  }

  /// A lookup table stretching [`Bins::clip_bounds`] to the full 0-255 range: values at or below the low bound map
  /// to 0, at or above the high bound to 255, and values between are spread linearly.
  pub fn levels_lut(&self, p_clip_fraction: f32) -> [u8; 256] {
    let (low, high) = self.clip_bounds(p_clip_fraction);
    let (low, high) = (low as i32, high as i32);
    let range = ((high - low) as f32).max(1.0);
    std::array::from_fn(|value| {
      let value = value as i32;
      let level = if value <= low {
        0.0
      } else if value >= high {
        1.0
      } else {
        (value - low) as f32 / range
      };
      (level * 255.0).round().clamp(0.0, 255.0) as u8
    })
  }
}

/// The first value, in `p_order`, at which the running count reaches `p_target`.
fn first_reaching(p_counts: &[u64; 256], p_order: impl Iterator<Item = usize>, p_target: u64) -> Option<usize> {
  let mut seen = 0u64;
  for value in p_order {
    seen += p_counts[value];
    if seen >= p_target {
      return Some(value);
    }
  }
  None
}

/// The value histograms of the red, green, blue, and alpha channels of an image.
#[derive(Debug, Clone, Default)]
pub struct Histogram {
  /// Boxed to keep the struct small on the stack.
  channels: Box<[Bins; 4]>,
}

impl Histogram {
  /// Creates an empty histogram.
  pub fn new() -> Self {
    Self::default()
  }

  /// Counts the channels of every pixel of an RGBA buffer.
  /// - `p_rgba`: The RGBA pixels.
  /// - `p_skip_transparent`: Whether fully transparent pixels are left out.
  ///
  /// ```ignore
  /// let histogram = Histogram::from_rgba(image.rgba(), true);
  /// let (low, high) = histogram.channel(Channel::R).clip_bounds(0.01);
  /// ```
  pub fn from_rgba(p_rgba: &[u8], p_skip_transparent: bool) -> Self {
    p_rgba
      .par_chunks_exact(4)
      .fold(Histogram::new, |mut histogram, pixel| {
        if !(p_skip_transparent && pixel[3] == 0) {
          for (bins, &value) in histogram.channels.iter_mut().zip(pixel) {
            bins.add(value);
          }
        }
        histogram
      })
      .reduce(Histogram::new, |mut a, b| {
        a.channels.iter_mut().zip(b.channels.iter()).for_each(|(bins_a, bins_b)| bins_a.merge(bins_b));
        a
      })
  }

  /// The bins of one channel.
  pub fn channel(&self, p_channel: Channel) -> &Bins {
    &self.channels[p_channel.index()]
  }

  /// The bins of one channel, for adding or removing values.
  pub fn channel_mut(&mut self, p_channel: Channel) -> &mut Bins {
    &mut self.channels[p_channel.index()]
  }

  /// Counts the red, green, and blue values of one pixel. Alpha is not counted.
  #[inline]
  pub fn add_rgb(&mut self, p_r: u8, p_g: u8, p_b: u8) {
    self.channels[0].add(p_r);
    self.channels[1].add(p_g);
    self.channels[2].add(p_b);
  }

  /// Removes every count.
  pub fn clear(&mut self) {
    self.channels.iter_mut().for_each(Bins::clear);
  }

  /// The number of pixels counted in the red channel.
  pub fn total_pixels(&self) -> u64 {
    self.channels[0].total()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn bins(p_values: &[u8]) -> Bins {
    let mut bins = Bins::new();
    p_values.iter().for_each(|&value| bins.add(value));
    bins
  }

  #[test]
  fn statistics() {
    let values = bins(&[10, 20, 30, 40, 50]);
    assert_eq!(values.median(), 30);
    assert_eq!(values.percentile(0.0), 10);
    assert_eq!(values.percentile(1.0), 50);
    assert_eq!(values.mean(), 30);
    assert_eq!(bins(&[7, 7, 3, 3, 9]).mode(), 3);
    assert_eq!(bins(&[10, 12, 200]).weighted_average(11, 5), 11);
    assert_eq!(Bins::new().median(), 0);
  }

  #[test]
  fn remove_keeps_the_total() {
    let mut values = bins(&[1, 2, 3]);
    values.remove(2);
    values.remove(99);
    assert_eq!(values.total(), 2);
    assert_eq!(values.median(), 3);
  }

  #[test]
  fn levels_stretch_the_clipped_range() {
    let values = bins(&[50, 100, 150]);
    assert_eq!(values.clip_bounds(0.0), (0, 255));
    assert_eq!(values.clip_bounds(0.34), (50, 150));
    let lut = values.levels_lut(0.34);
    assert_eq!((lut[50], lut[100], lut[150]), (0, 128, 255));
  }

  #[test]
  fn histogram_skips_transparent_pixels() {
    let pixels = [10, 20, 30, 255, 40, 50, 60, 0];
    assert_eq!(Histogram::from_rgba(&pixels, false).total_pixels(), 2);
    let histogram = Histogram::from_rgba(&pixels, true);
    assert_eq!(histogram.total_pixels(), 1);
    assert_eq!(histogram.channel(Channel::G).median(), 20);
  }
}
