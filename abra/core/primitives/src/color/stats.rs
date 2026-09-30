use rayon::prelude::*;

use super::Color;
use super::histogram::Bins;
use crate::channels::Channels;

/// A statistic used to reduce many pixels to a single color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorStat {
  /// The per-channel mean.
  Average,
  /// The per-channel median.
  Median,
  /// The per-channel most frequent value.
  Mode,
}

impl Color {
  /// Reduces a pixel buffer to a single opaque color using the given statistic.
  ///
  /// Each of the red, green, and blue channels is reduced independently; alpha input is ignored
  /// and the result is always opaque. An empty buffer returns [`Color::transparent`].
  ///
  /// - `p_pixels`: The pixel buffer. A trailing partial pixel is ignored.
  /// - `p_channels`: The channel layout of `p_pixels`.
  /// - `p_stat`: The statistic to compute.
  ///
  /// ```ignore
  /// let average = Color::from_pixels(image.rgba(), Channels::RGBA, ColorStat::Average);
  /// ```
  pub fn from_pixels(p_pixels: &[u8], p_channels: Channels, p_stat: ColorStat) -> Color {
    let stride = p_channels.bytes_per_pixel();
    let count = p_pixels.len() / stride;
    if count == 0 {
      return Color::transparent();
    }

    let (r, g, b) = match p_stat {
      ColorStat::Average => {
        let (r, g, b) = p_pixels
          .par_chunks_exact(stride)
          .fold(|| (0u64, 0u64, 0u64), |(r, g, b), px| (r + px[0] as u64, g + px[1] as u64, b + px[2] as u64))
          .reduce(|| (0, 0, 0), |(r1, g1, b1), (r2, g2, b2)| (r1 + r2, g1 + g2, b1 + b2));
        let count = count as u64;
        ((r / count) as u8, (g / count) as u8, (b / count) as u8)
      }
      ColorStat::Median => {
        let [r, g, b] = rgb_bins(p_pixels, stride);
        (r.median(), g.median(), b.median())
      }
      ColorStat::Mode => {
        let [r, g, b] = rgb_bins(p_pixels, stride);
        (r.mode(), g.mode(), b.mode())
      }
    };
    Color::from_rgb(r, g, b)
  }
}

/// The value bins of the red, green, and blue channels.
fn rgb_bins(p_pixels: &[u8], p_stride: usize) -> [Bins; 3] {
  p_pixels
    .par_chunks_exact(p_stride)
    .fold(
      || [Bins::new(), Bins::new(), Bins::new()],
      |mut bins, px| {
        bins.iter_mut().zip(px).for_each(|(bins, &value)| bins.add(value));
        bins
      },
    )
    .reduce(
      || [Bins::new(), Bins::new(), Bins::new()],
      |mut a, b| {
        a.iter_mut().zip(b.iter()).for_each(|(bins_a, bins_b)| bins_a.merge(bins_b));
        a
      },
    )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn average_of_rgb_buffer_that_is_a_multiple_of_four() {
    // 4 RGB pixels = 12 bytes, which used to be misread as 3 RGBA pixels.
    let pixels = [0, 0, 0, 100, 100, 100, 200, 200, 200, 100, 100, 100];
    assert_eq!(Color::from_pixels(&pixels, Channels::RGB, ColorStat::Average).rgb(), (100, 100, 100));
  }

  #[test]
  fn median_of_rgba_buffer() {
    let pixels = [
      10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255, 50, 0, 0, 255,
    ];
    assert_eq!(Color::from_pixels(&pixels, Channels::RGBA, ColorStat::Median).r, 30);
  }

  #[test]
  fn mode_picks_most_frequent_value_per_channel() {
    let pixels = [5, 9, 1, 7, 9, 1, 7, 2, 1];
    assert_eq!(Color::from_pixels(&pixels, Channels::RGB, ColorStat::Mode).rgb(), (7, 9, 1));
  }

  #[test]
  fn empty_buffer_is_transparent() {
    assert_eq!(Color::from_pixels(&[], Channels::RGBA, ColorStat::Average), Color::transparent());
  }
}
