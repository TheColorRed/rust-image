//! Interpolation algorithms for image sampling.
//!
//! This module provides the interpolation methods used for sampling pixels at non-integer coordinates, and
//! [`remap`], the shared loop every geometric transform (resize, rotate, warp, distortions) is built on: each output
//! pixel is mapped back to a position in the source image and sampled there.

use primitives::{Image, LumaStandard, luma};
use rayon::prelude::*;

/// Sampling algorithm used by interpolation and resampling operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
  /// The pixel whose center is closest. Fastest, blocky.
  Nearest,
  /// Blends the 2x2 neighborhood.
  Bilinear,
  /// Cubic kernel over the 4x4 neighborhood.
  Bicubic,
  /// Lanczos-3 kernel over the 6x6 neighborhood. Sharpest, slowest.
  Lanczos,
  /// New Edge-Directed Interpolation: follows the local edge direction found from the gradient covariance of a 5x5
  /// window, and uses bicubic where there is no strong edge.
  EdgeDirectedNedi,
  /// Edge-Directed Interpolation: follows the Sobel edge direction snapped to 45 degrees, and uses bilinear where
  /// there is no strong edge. Faster than NEDI.
  EdgeDirectedEdi,
}

/// What a sampler reads for pixels past the edges of the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EdgeMode {
  /// Pixels past the edges are fully transparent, so edges fade out rather than turning an opaque color. Default.
  #[default]
  Transparent,
  /// Pixels past the edges repeat the nearest edge pixel, so edges keep their color and opacity.
  Clamp,
}

/// Sample a pixel with the selected interpolation algorithm.
/// # Arguments
/// - `p_image`: The image to sample from.
/// - `p_x`, `p_y`: The position to sample, where pixel `(0, 0)` is centered on `(0.0, 0.0)`.
/// - `p_interpolation`: The interpolation algorithm.
/// - `p_edge`: What the sampler reads past the edges of the image.
pub fn sample(p_image: &Image, p_x: f32, p_y: f32, p_interpolation: Interpolation, p_edge: EdgeMode) -> [u8; 4] {
  let source = Source::new(p_image, p_edge);
  match p_interpolation {
    Interpolation::Nearest => source.fetch(p_x.round() as i32, p_y.round() as i32),
    Interpolation::Bilinear => sample_kernel(&source, p_x, p_y, 0, 1, tent),
    Interpolation::Bicubic => sample_kernel(&source, p_x, p_y, -1, 2, cubic),
    Interpolation::Lanczos => sample_kernel(&source, p_x, p_y, -2, 3, lanczos3),
    Interpolation::EdgeDirectedNedi => sample_nedi(&source, p_x, p_y),
    Interpolation::EdgeDirectedEdi => sample_edi(&source, p_x, p_y),
  }
}

/// Builds a `p_width` x `p_height` RGBA buffer by mapping each output pixel back to a position in `p_source` and
/// sampling it there.
///
/// `p_map` receives the center of an output pixel in continuous coordinates, where pixel `i` spans `i..i + 1` so
/// its center is `i + 0.5`, and returns the matching continuous position in the source. Returning `None` leaves the
/// pixel transparent, for positions the source does not cover.
///
/// ```ignore
/// // Scale by 2 around the origin.
/// let pixels = remap(&image, width * 2, height * 2, Interpolation::Bicubic, EdgeMode::Clamp, |x, y| Some((x / 2.0, y / 2.0)));
/// ```
pub fn remap<F>(
  p_source: &Image, p_width: u32, p_height: u32, p_interpolation: Interpolation, p_edge: EdgeMode, p_map: F,
) -> Vec<u8>
where
  F: Fn(f64, f64) -> Option<(f64, f64)> + Sync,
{
  let buffer_size = (p_width as u64)
    .checked_mul(p_height as u64)
    .and_then(|size| size.checked_mul(4))
    .expect("Image dimensions too large") as usize;
  let mut pixels = vec![0u8; buffer_size];
  if p_width == 0 {
    return pixels;
  }
  let source = Source::new(p_source, p_edge);
  pixels.par_chunks_exact_mut(p_width as usize * 4).enumerate().for_each(|(y, row)| {
    let center_y = y as f64 + 0.5;
    for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
      if let Some((source_x, source_y)) = p_map(x as f64 + 0.5, center_y) {
        let (sx, sy) = ((source_x - 0.5) as f32, (source_y - 0.5) as f32);
        let sampled = match p_interpolation {
          Interpolation::Nearest => source.fetch(sx.round() as i32, sy.round() as i32),
          Interpolation::Bilinear => sample_kernel(&source, sx, sy, 0, 1, tent),
          Interpolation::Bicubic => sample_kernel(&source, sx, sy, -1, 2, cubic),
          Interpolation::Lanczos => sample_kernel(&source, sx, sy, -2, 3, lanczos3),
          Interpolation::EdgeDirectedNedi => sample_nedi(&source, sx, sy),
          Interpolation::EdgeDirectedEdi => sample_edi(&source, sx, sy),
        };
        pixel.copy_from_slice(&sampled);
      }
    }
  });
  pixels
}

/// Resample an image to a new size with the selected interpolation algorithm.
pub fn resample(p_source: &Image, p_width: u32, p_height: u32, p_interpolation: Interpolation) -> Vec<u8> {
  let (old_width, old_height) = p_source.dimensions::<u32>();
  let scale_x = old_width as f64 / p_width as f64;
  let scale_y = old_height as f64 / p_height as f64;
  remap(p_source, p_width, p_height, p_interpolation, EdgeMode::Transparent, |x, y| Some((x * scale_x, y * scale_y)))
}

/// The pixels of an image with the edge handling used to read past its borders.
struct Source<'a> {
  pixels: &'a [u8],
  width: u32,
  height: u32,
  edge: EdgeMode,
}

impl<'a> Source<'a> {
  fn new(p_image: &'a Image, p_edge: EdgeMode) -> Self {
    let (width, height) = p_image.dimensions::<u32>();
    Source {
      pixels: p_image.rgba(),
      width,
      height,
      edge: p_edge,
    }
  }

  /// The pixel at `(p_x, p_y)`, with positions past the edges read as the edge mode says.
  #[inline]
  fn fetch(&self, p_x: i32, p_y: i32) -> [u8; 4] {
    if self.width == 0 || self.height == 0 {
      return [0, 0, 0, 0];
    }
    let (x, y) = match self.edge {
      EdgeMode::Transparent => {
        if p_x < 0 || p_y < 0 || p_x >= self.width as i32 || p_y >= self.height as i32 {
          return [0, 0, 0, 0];
        }
        (p_x as usize, p_y as usize)
      }
      EdgeMode::Clamp => (p_x.clamp(0, self.width as i32 - 1) as usize, p_y.clamp(0, self.height as i32 - 1) as usize),
    };
    let index = (y * self.width as usize + x) * 4;
    match self.pixels.get(index..index + 4) {
      Some(pixel) => [pixel[0], pixel[1], pixel[2], pixel[3]],
      None => [0, 0, 0, 0],
    }
  }

  /// The pixel with its color premultiplied by alpha, as `[r*a, g*a, b*a, a]` with `a` in 0-1.
  #[inline]
  fn premultiplied(&self, p_x: i32, p_y: i32) -> [f32; 4] {
    let pixel = self.fetch(p_x, p_y);
    let a = pixel[3] as f32 / 255.0;
    [pixel[0] as f32 * a, pixel[1] as f32 * a, pixel[2] as f32 * a, a]
  }

  /// The bilinear blend of premultiplied pixels at a fractional position.
  fn premultiplied_bilinear(&self, p_x: f32, p_y: f32) -> [f32; 4] {
    let (x0, y0) = (p_x.floor() as i32, p_y.floor() as i32);
    let (fx, fy) = (p_x - x0 as f32, p_y - y0 as f32);
    let top = lerp4(self.premultiplied(x0, y0), self.premultiplied(x0 + 1, y0), fx);
    let bottom = lerp4(self.premultiplied(x0, y0 + 1), self.premultiplied(x0 + 1, y0 + 1), fx);
    lerp4(top, bottom, fy)
  }

  /// The luma of a premultiplied pixel, un-premultiplied.
  #[inline]
  fn luma(&self, p_x: i32, p_y: i32) -> f32 {
    let [r, g, b, a] = self.premultiplied(p_x, p_y);
    if a > 0.0 { luma(r, g, b, LumaStandard::Rec601) / a } else { 0.0 }
  }
}

/// Linear interpolation between two premultiplied pixels.
#[inline]
fn lerp4(p_a: [f32; 4], p_b: [f32; 4], p_t: f32) -> [f32; 4] {
  [0, 1, 2, 3].map(|i| p_a[i] + (p_b[i] - p_a[i]) * p_t)
}

/// Converts an accumulated premultiplied pixel back to straight 8-bit RGBA.
#[inline]
fn unpremultiply(p_acc: [f32; 4]) -> [u8; 4] {
  let [r, g, b, a] = p_acc;
  if a <= 0.0 {
    return [0, 0, 0, 0];
  }
  let channel = |value: f32| (value / a).clamp(0.0, 255.0).round() as u8;
  [
    channel(r),
    channel(g),
    channel(b),
    (a * 255.0).clamp(0.0, 255.0).round() as u8,
  ]
}

/// Weighted sum of premultiplied pixels in the window `x0 + p_from..=x0 + p_to` (same for y) around the sample,
/// with each pixel weighted by `p_kernel(dx) * p_kernel(dy)` and the result normalized by the total weight.
#[inline]
fn sample_kernel(p_source: &Source, p_x: f32, p_y: f32, p_from: i32, p_to: i32, p_kernel: fn(f32) -> f32) -> [u8; 4] {
  let (x0, y0) = (p_x.floor() as i32, p_y.floor() as i32);
  let (fx, fy) = (p_x - x0 as f32, p_y - y0 as f32);
  let mut acc = [0.0f32; 4];
  let mut weight_sum = 0.0;
  for dy in p_from..=p_to {
    let wy = p_kernel(dy as f32 - fy);
    for dx in p_from..=p_to {
      let w = p_kernel(dx as f32 - fx) * wy;
      let pixel = p_source.premultiplied(x0 + dx, y0 + dy);
      for i in 0..4 {
        acc[i] += pixel[i] * w;
      }
      weight_sum += w;
    }
  }
  if weight_sum > 0.0 {
    acc = acc.map(|value| value / weight_sum);
  }
  unpremultiply(acc)
}

/// Linear (tent) kernel: the bilinear weights.
fn tent(p_t: f32) -> f32 {
  (1.0 - p_t.abs()).max(0.0)
}

/// Cubic convolution kernel.
fn cubic(p_t: f32) -> f32 {
  let t = p_t.abs();
  if t < 1.0 {
    1.0 - 2.0 * t * t + t * t * t
  } else if t < 2.0 {
    -4.0 + 8.0 * t - 5.0 * t * t + t * t * t
  } else {
    0.0
  }
}

/// Lanczos kernel with a = 3.
fn lanczos3(p_t: f32) -> f32 {
  const A: f32 = 3.0;
  let t = p_t.abs();
  if t == 0.0 {
    1.0
  } else if t < A {
    let pi_t = std::f32::consts::PI * t;
    (pi_t.sin() / pi_t) * ((pi_t / A).sin() / (pi_t / A))
  } else {
    0.0
  }
}

/// New Edge-Directed Interpolation. Estimates the edge direction from the covariance of luma gradients in a 5x5
/// window and interpolates along it; falls back to bicubic where the edge is weak.
fn sample_nedi(p_source: &Source, p_x: f32, p_y: f32) -> [u8; 4] {
  const WINDOW: i32 = 2;
  const EDGE_THRESHOLD: f32 = 5.0;

  let (x0, y0) = (p_x.floor() as i32, p_y.floor() as i32);
  let (fx, fy) = (p_x - x0 as f32, p_y - y0 as f32);

  let mut gradients = [(0.0f32, 0.0f32); ((2 * WINDOW + 1) * (2 * WINDOW + 1)) as usize];
  let mut index = 0;
  for dy in -WINDOW..=WINDOW {
    for dx in -WINDOW..=WINDOW {
      let (px, py) = (x0 + dx, y0 + dy);
      let gx = (p_source.luma(px + 1, py) - p_source.luma(px - 1, py)) * 0.5;
      let gy = (p_source.luma(px, py + 1) - p_source.luma(px, py - 1)) * 0.5;
      gradients[index] = (gx, gy);
      index += 1;
    }
  }

  let count = gradients.len() as f32;
  let (mean_x, mean_y) = gradients.iter().fold((0.0, 0.0), |(sx, sy), (gx, gy)| (sx + gx, sy + gy));
  let (mean_x, mean_y) = (mean_x / count, mean_y / count);
  let (mut a, mut b, mut c) = (0.0f32, 0.0f32, 0.0f32);
  for (gx, gy) in gradients {
    let (dx, dy) = (gx - mean_x, gy - mean_y);
    a += dx * dx;
    b += dx * dy;
    c += dy * dy;
  }
  let (a, b, c) = (a / count, b / count, c / count);

  if (a + c).sqrt() <= EDGE_THRESHOLD {
    return sample_kernel(p_source, p_x, p_y, -1, 2, cubic);
  }

  // The eigenvector of the larger eigenvalue of the covariance is the main edge direction.
  let trace = a + c;
  let discriminant = (trace * trace * 0.25 - (a * c - b * b)).max(0.0).sqrt();
  let (lambda1, lambda2) = (trace * 0.5 + discriminant, trace * 0.5 - discriminant);
  let lambda = if lambda1.abs() > lambda2.abs() { lambda1 } else { lambda2 };
  let (edge_x, edge_y) = if b.abs() > 1e-6 {
    let (vx, vy) = (lambda - c, b);
    let norm = (vx * vx + vy * vy).sqrt();
    if norm > 0.0 { (vx / norm, vy / norm) } else { (1.0, 0.0) }
  } else if (a - c).abs() > 1e-6 {
    if a > c { (1.0, 0.0) } else { (0.0, 1.0) }
  } else {
    (1.0, 0.0)
  };

  let behind = p_source.premultiplied_bilinear(x0 as f32 - edge_x, y0 as f32 - edge_y);
  let ahead = p_source.premultiplied_bilinear(x0 as f32 + edge_x, y0 as f32 + edge_y);
  let t = ((fx * edge_x + fy * edge_y) + 1.0) * 0.5;
  unpremultiply(lerp4(behind, ahead, t))
}

/// Edge-Directed Interpolation. Finds the Sobel edge direction, snaps it to one of four directions, and
/// interpolates between the two pixels along it; falls back to bilinear where the edge is weak.
fn sample_edi(p_source: &Source, p_x: f32, p_y: f32) -> [u8; 4] {
  const EDGE_THRESHOLD: f32 = 10.0;

  let (x0, y0) = (p_x.floor() as i32, p_y.floor() as i32);
  let (fx, fy) = (p_x - x0 as f32, p_y - y0 as f32);
  let l = |dx: i32, dy: i32| p_source.luma(x0 + dx, y0 + dy);

  let gx = -l(-1, -1) - 2.0 * l(-1, 0) - l(-1, 1) + l(1, -1) + 2.0 * l(1, 0) + l(1, 1);
  let gy = -l(-1, -1) - 2.0 * l(0, -1) - l(1, -1) + l(-1, 1) + 2.0 * l(0, 1) + l(1, 1);

  if (gx * gx + gy * gy).sqrt() <= EDGE_THRESHOLD {
    return sample_kernel(p_source, p_x, p_y, 0, 1, tent);
  }

  let angle = gy.atan2(gx);
  let angle = if angle < 0.0 { angle + std::f32::consts::PI } else { angle };
  let direction = ((angle / std::f32::consts::PI * 4.0).round() as i32) % 4;
  let ((ax, ay), (bx, by), t) = match direction {
    0 => ((0, 0), (1, 0), fx),
    1 => ((0, 0), (1, 1), (fx + fy) * 0.5),
    2 => ((0, 0), (0, 1), fy),
    _ => ((1, 0), (0, 1), (fx + fy) * 0.5),
  };
  let start = p_source.premultiplied(x0 + ax, y0 + ay);
  let end = p_source.premultiplied(x0 + bx, y0 + by);
  unpremultiply(lerp4(start, end, t))
}

#[cfg(test)]
mod tests {
  use super::*;
  use primitives::Channels;

  fn checker(p_size: u32) -> Image {
    let pixels = (0..p_size * p_size)
      .flat_map(|i| if (i % p_size + i / p_size) % 2 == 0 { [255, 255, 255, 255] } else { [0, 0, 0, 255] })
      .collect::<Vec<u8>>();
    Image::new_from_pixels(p_size, p_size, pixels, Channels::RGBA)
  }

  #[test]
  fn every_interpolation_returns_the_pixel_at_its_center() {
    let image = checker(8);
    for interpolation in [
      Interpolation::Nearest,
      Interpolation::Bilinear,
      Interpolation::EdgeDirectedEdi,
    ] {
      for (x, y) in [(2, 2), (3, 2), (4, 5)] {
        let expected = image.get_pixel(x, y).unwrap();
        let sampled = sample(&image, x as f32, y as f32, interpolation, EdgeMode::Clamp);
        assert_eq!(sampled, [expected.0, expected.1, expected.2, expected.3], "{interpolation:?} at {x},{y}");
      }
    }
  }

  #[test]
  fn resampling_to_the_same_size_keeps_the_image() {
    let image = checker(6);
    for interpolation in [Interpolation::Nearest, Interpolation::Bilinear] {
      assert_eq!(resample(&image, 6, 6, interpolation), image.rgba(), "{interpolation:?}");
    }
  }

  #[test]
  fn remap_leaves_unmapped_pixels_transparent() {
    let image = Image::new_from_color(4, 4, primitives::Color::red());
    let pixels = remap(&image, 4, 1, Interpolation::Nearest, EdgeMode::Clamp, |x, y| (x < 2.0).then_some((x, y)));
    assert_eq!(&pixels[..4], &[255, 0, 0, 255]);
    assert_eq!(&pixels[8..12], &[0, 0, 0, 0]);
  }
}
