use crate::Image;
use rayon::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlipAxis {
  Horizontal,
  Vertical,
}

/// A flip that has been described but not yet run. Create one with [`flip`], then run it with [`FlipImage::apply`].
pub struct FlipImage {
  pub axis: FlipAxis,
}

impl FlipImage {
  /// Flips the image in place.
  pub fn apply(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<usize>();
    if width == 0 || height == 0 {
      return;
    }
    let stride = width * 4;
    let source = p_image.rgba();
    let mut pixels = vec![0u8; source.len()];
    pixels.par_chunks_exact_mut(stride).enumerate().for_each(|(y, row)| match self.axis {
      FlipAxis::Horizontal => {
        let source_row = &source[y * stride..(y + 1) * stride];
        for (pixel, source_pixel) in row.chunks_exact_mut(4).zip(source_row.chunks_exact(4).rev()) {
          pixel.copy_from_slice(source_pixel);
        }
      }
      FlipAxis::Vertical => {
        let source_y = height - 1 - y;
        row.copy_from_slice(&source[source_y * stride..(source_y + 1) * stride]);
      }
    });
    p_image.set_rgba(pixels);
  }
}

/// Flips an image along the requested axis.
/// # Arguments
/// - `p_axis`: The axis to mirror the image across.
pub fn flip(p_axis: FlipAxis) -> FlipImage {
  FlipImage { axis: p_axis }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Channels;

  #[test]
  fn flips_mirror_the_image() {
    let pixels: Vec<u8> = (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect();
    let mut image = Image::new_from_pixels(3, 2, pixels.clone(), Channels::RGBA);
    flip(FlipAxis::Horizontal).apply(&mut image);
    assert_eq!(image.get_pixel(0, 0), Some((2, 0, 0, 255)));
    let mut image = Image::new_from_pixels(3, 2, pixels, Channels::RGBA);
    flip(FlipAxis::Vertical).apply(&mut image);
    assert_eq!(image.get_pixel(0, 0), Some((3, 0, 0, 255)));
  }
}
