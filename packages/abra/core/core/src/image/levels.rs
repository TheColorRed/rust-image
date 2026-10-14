//! An image at half its size, a quarter, an eighth and so on: the zoomed-out versions a view needs, so it never has to read
//! every pixel of a large photo to draw a small picture of it.
//!
//! Level `0` is the image itself and level `n` is the image halved `n` times, each halving the mean of the up to four pixels it
//! covers, rounded up in size when a side is odd. A level is made the first time it is asked for, from the one above it, and
//! kept with the image until its pixels change, so an image that is only ever read at its own size costs nothing for the rest.

use std::sync::{Arc, Mutex};

use rayon::prelude::*;

use crate::Image;

/// One level of an image: its pixels, and its size.
pub(crate) struct Level {
  pub(crate) width: u32,
  pub(crate) height: u32,
  pub(crate) pixels: Vec<u8>,
}

/// The levels of an image above the first, as far as anybody has asked for them. Kept with the image (see [`Image::derived`]).
#[derive(Default)]
pub(crate) struct Levels {
  made: Mutex<Vec<Arc<Level>>>,
}

/// The size of an image `p_width` x `p_height` at `p_level`: halved that many times, rounding up, and never below one pixel.
pub fn level_size(p_width: u32, p_height: u32, p_level: u32) -> (u32, u32) {
  let (mut width, mut height) = (p_width.max(1), p_height.max(1));
  for _ in 0..p_level {
    (width, height) = (width.div_ceil(2), height.div_ceil(2));
  }
  (width, height)
}

/// A distance of `p_pixels` in the image, as it is at `p_level`: half as long for each halving. An effect that looks at the pixels
/// around it uses this for its radius, so it looks the same at any level.
pub fn scaled_to_level(p_pixels: f32, p_level: u32) -> f32 {
  p_pixels / 2f32.powi(p_level.min(31) as i32)
}

impl Levels {
  /// The pixels of `p_image` at `p_level`, which must be above 0, making it and the levels between first if they are not there.
  pub(crate) fn level(&self, p_image: &Image, p_level: u32) -> Arc<Level> {
    let mut made = self.made.lock().unwrap();
    while (made.len() as u32) < p_level {
      let next = match made.last() {
        Some(last) => halved(last.width, last.height, &last.pixels),
        None => {
          let (width, height) = p_image.dimensions::<u32>();
          halved(width, height, p_image.rgba())
        }
      };
      made.push(Arc::new(next));
    }
    Arc::clone(&made[p_level as usize - 1])
  }
}

/// `p_pixels`, `p_width` x `p_height` RGBA, at half the size: each pixel the mean of the up to four it covers, in every channel.
fn halved(p_width: u32, p_height: u32, p_pixels: &[u8]) -> Level {
  let (width, height) = (p_width.div_ceil(2).max(1), p_height.div_ceil(2).max(1));
  let mut pixels = vec![0u8; width as usize * height as usize * 4];
  if p_width == 0 || p_height == 0 {
    return Level { width, height, pixels };
  }
  pixels.par_chunks_exact_mut(width as usize * 4).enumerate().for_each(|(y, row)| {
    for x in 0..width as usize {
      let mut sum = [0u32; 4];
      let mut count = 0;
      for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let (source_x, source_y) = (x * 2 + dx, y * 2 + dy);
        if source_x < p_width as usize && source_y < p_height as usize {
          let at = (source_y * p_width as usize + source_x) * 4;
          for channel in 0..4 {
            sum[channel] += p_pixels[at + channel] as u32;
          }
          count += 1;
        }
      }
      for channel in 0..4 {
        row[x * 4 + channel] = ((sum[channel] + count / 2) / count) as u8;
      }
    }
  });
  Level { width, height, pixels }
}

/// The rectangle `p_width` x `p_height` of `p_pixels` (`p_source_width` x `p_source_height` RGBA) whose top left is at
/// `(p_left, p_top)`, as RGBA. A pixel past an edge of the source is the pixel at that edge.
pub(crate) fn copy_clamped(
  p_pixels: &[u8], p_source_width: u32, p_source_height: u32, p_left: i32, p_top: i32, p_width: u32, p_height: u32,
) -> Vec<u8> {
  let mut out = vec![0u8; p_width as usize * p_height as usize * 4];
  if p_source_width == 0 || p_source_height == 0 || p_width == 0 {
    return out;
  }
  let (source_width, source_height) = (p_source_width as i64, p_source_height as i64);
  // The columns of the rectangle that are inside the source run from `from` up to, not including, `to`.
  let from = (-(p_left as i64)).clamp(0, p_width as i64) as usize;
  let to = (source_width - p_left as i64).clamp(from as i64, p_width as i64) as usize;
  out.par_chunks_exact_mut(p_width as usize * 4).enumerate().for_each(|(row, line)| {
    let source_y = (p_top as i64 + row as i64).clamp(0, source_height - 1) as usize;
    let source = &p_pixels[source_y * p_source_width as usize * 4..(source_y + 1) * p_source_width as usize * 4];
    for column in 0..from {
      line[column * 4..column * 4 + 4].copy_from_slice(&source[..4]);
    }
    if to > from {
      let start = (p_left as i64 + from as i64) as usize * 4;
      line[from * 4..to * 4].copy_from_slice(&source[start..start + (to - from) * 4]);
    }
    for column in to..p_width as usize {
      line[column * 4..column * 4 + 4].copy_from_slice(&source[source.len() - 4..]);
    }
  });
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Channels;

  /// A `p_width` x `p_height` image whose red is its column, green its row, and blue and alpha 100 and 255.
  fn ramp(p_width: u32, p_height: u32) -> Image {
    let mut pixels = Vec::new();
    for y in 0..p_height {
      for x in 0..p_width {
        pixels.extend_from_slice(&[x as u8, y as u8, 100, 255]);
      }
    }
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  #[test]
  fn each_level_is_half_the_size_of_the_one_above_and_rounds_up() {
    assert_eq!(level_size(8000, 6000, 0), (8000, 6000));
    assert_eq!(level_size(8000, 6000, 3), (1000, 750));
    assert_eq!(level_size(5, 3, 1), (3, 2));
    assert_eq!(level_size(5, 3, 2), (2, 1));
    assert_eq!(level_size(5, 3, 9), (1, 1), "never below one pixel");
  }

  #[test]
  fn a_halved_level_is_the_mean_of_each_block_and_the_odd_edge_is_the_mean_of_what_is_there() {
    let image = ramp(5, 3);
    let levels = Levels::default();
    let level = levels.level(&image, 1);
    assert_eq!((level.width, level.height), (3, 2));
    // The block of columns 0 and 1 and rows 0 and 1: red is 0.5 on average, green 0.5; rounded half up.
    assert_eq!(&level.pixels[..4], &[1, 1, 100, 255]);
    // The last column (4) has no neighbour, so the block is the two pixels of that column: red 4, green 0.5.
    assert_eq!(&level.pixels[2 * 4..2 * 4 + 4], &[4, 1, 100, 255]);
    // The last row (2) has no row under it: red of columns 0 and 1 is 0.5, green 2.
    assert_eq!(&level.pixels[3 * 4..3 * 4 + 4], &[1, 2, 100, 255]);
    // And the one above is the same level, not made again.
    assert!(Arc::ptr_eq(&levels.level(&image, 1), &level));
    let quarter = levels.level(&image, 2);
    assert_eq!((quarter.width, quarter.height), (2, 1));
  }

  #[test]
  fn a_rectangle_is_copied_and_what_is_past_an_edge_is_the_pixel_at_that_edge() {
    let image = ramp(4, 3);
    let inside = copy_clamped(image.rgba(), 4, 3, 1, 1, 2, 2);
    assert_eq!(inside, vec![1, 1, 100, 255, 2, 1, 100, 255, 1, 2, 100, 255, 2, 2, 100, 255]);
    // One pixel past the top left corner: the corner repeated along the top and down the left.
    let corner = copy_clamped(image.rgba(), 4, 3, -1, -1, 3, 3);
    let pixel = |x: usize, y: usize| corner[(y * 3 + x) * 4..(y * 3 + x) * 4 + 4].to_vec();
    assert_eq!(pixel(0, 0), vec![0, 0, 100, 255]);
    assert_eq!(pixel(1, 0), vec![0, 0, 100, 255]);
    assert_eq!(pixel(0, 1), vec![0, 0, 100, 255]);
    assert_eq!(pixel(2, 2), vec![1, 1, 100, 255]);
    // Past the bottom right corner, and a rectangle with nothing of the source in it.
    let far = copy_clamped(image.rgba(), 4, 3, 10, 10, 2, 1);
    assert_eq!(far, vec![3, 2, 100, 255, 3, 2, 100, 255]);
  }
}
