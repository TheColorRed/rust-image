use crate::kernel::apply_kernel;
use abra_core::Image;
use abra_core::color::{LumaStandard, luma};
use options::{Effect, Options};
use rayon::prelude::*;

/// How strong the edges are at each pixel, from the image's brightness, using the Sobel kernels in both directions.
///
/// Unlike the [`sobel`] filter this keeps every edge, whichever way it goes, and does not round to 8 bits. The values
/// are in levels of brightness: a straight edge where the brightness jumps by `d` levels measures `d` along it, so a
/// threshold means the same thing on every image. Pixels past the border use the nearest pixel inside.
///
/// With a `p_step` above 1 the kernels' taps are that many pixels apart, so edges are measured as they would be on a
/// photo `p_step` times smaller. Use it to measure a large photo at the scale a threshold was chosen for, without also
/// multiplying its grain.
/// - `p_step`: The distance between the kernels' taps, in pixels. `1` is the plain Sobel kernels.
/// Returns `width * height` values, row by row.
pub fn sobel_magnitude(p_image: &Image, p_step: usize) -> Vec<f32> {
  let (w, h) = p_image.dimensions::<usize>();
  if w == 0 || h == 0 {
    return Vec::new();
  }
  let rgba = p_image.rgba();
  let brightness: Vec<f32> =
    rgba.chunks_exact(4).map(|p| luma(p[0] as f32, p[1] as f32, p[2] as f32, LumaStandard::Rec601)).collect();
  let at = |x: isize, y: isize| brightness[y.clamp(0, h as isize - 1) as usize * w + x.clamp(0, w as isize - 1) as usize];
  let step = p_step.max(1) as isize;

  let mut out = vec![0.0f32; w * h];
  out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
    let y = y as isize;
    for (x, value) in row.iter_mut().enumerate() {
      let x = x as isize;
      let gx = (at(x + step, y - step) + 2.0 * at(x + step, y) + at(x + step, y + step))
        - (at(x - step, y - step) + 2.0 * at(x - step, y) + at(x - step, y + step));
      let gy = (at(x - step, y + step) + 2.0 * at(x, y + step) + at(x + step, y + step))
        - (at(x - step, y - step) + 2.0 * at(x, y - step) + at(x + step, y - step));
      // The kernels add up to 4 across an edge, so dividing by 4 gives the size of the jump.
      *value = (gx * gx + gy * gy).sqrt() / 4.0;
    }
  });
  out
}

/// Direction of the Sobel derivative kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SobelDirection {
  Horizontal,
  Vertical,
}

/// Applies the Sobel filter in the requested direction.
fn apply_sobel(p_image: &mut Image, p_direction: SobelDirection) {
  #[rustfmt::skip]
  let kernel = match p_direction {
    SobelDirection::Horizontal => &[1.0, 2.0, 1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0],
    SobelDirection::Vertical => &[1.0, 0.0, -1.0, 2.0, 0.0, -2.0, 1.0, 0.0, -1.0],
  };
  apply_kernel(p_image, kernel);
}
#[derive(Clone)]
pub struct Sobel {
  direction: SobelDirection,
  options: Options,
}
impl Effect for Sobel {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }
  fn padding(&self) -> i32 {
    1
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_sobel(p_image, self.direction);
  }
}
pub fn sobel(p_direction: SobelDirection) -> Sobel {
  Sobel {
    direction: p_direction,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Area, Image};
  use options::ApplyOptions;

  #[test]
  fn magnitude_is_zero_on_a_flat_image_and_the_size_of_the_jump_on_a_straight_edge() {
    let mut img = Image::new(8, 6);
    for y in 0..6u32 {
      for x in 0..8u32 {
        // Dark on the left, 200 levels brighter on the right.
        let level = if x < 4 { 20u8 } else { 220u8 };
        img.set_pixel(x, y, (level, level, level, 255));
      }
    }
    let magnitude = sobel_magnitude(&img, 1);
    assert_eq!(magnitude.len(), 8 * 6);
    // Far from the edge nothing changes.
    assert!(magnitude[2 * 8 + 1] < 0.001 && magnitude[2 * 8 + 6] < 0.001);
    // On either side of the edge it measures the jump, whichever way the brightness goes.
    assert!((magnitude[2 * 8 + 3] - 200.0).abs() < 0.5, "left of the edge: {}", magnitude[2 * 8 + 3]);
    assert!((magnitude[2 * 8 + 4] - 200.0).abs() < 0.5, "right of the edge: {}", magnitude[2 * 8 + 4]);

    // The same edge facing the other way measures the same.
    let mut flipped = Image::new(8, 6);
    for y in 0..6u32 {
      for x in 0..8u32 {
        let level = if x < 4 { 220u8 } else { 20u8 };
        flipped.set_pixel(x, y, (level, level, level, 255));
      }
    }
    assert!((sobel_magnitude(&flipped, 1)[2 * 8 + 3] - 200.0).abs() < 0.5);
  }

  #[test]
  fn a_larger_step_measures_a_ramp_as_if_the_photo_were_smaller() {
    // Brightness climbs 10 levels per pixel along the row. A photo twice as small would climb 20 per pixel.
    let mut img = Image::new(32, 8);
    for y in 0..8u32 {
      for x in 0..32u32 {
        let level = (x * 8) as u8;
        img.set_pixel(x, y, (level, level, level, 255));
      }
    }
    let at = |step: usize| sobel_magnitude(&img, step)[4 * 32 + 12];
    // A step edge of `d` measures `d`, so a ramp measures the climb across the taps: 2 * slope * step.
    assert!((at(1) - 16.0).abs() < 0.5, "step 1: {}", at(1));
    assert!((at(2) - 32.0).abs() < 0.5, "step 2: {}", at(2));
    assert!((at(3) - 48.0).abs() < 0.5, "step 3: {}", at(3));
  }

  #[test]
  fn sobel_area_respects_apply_options() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 255u8, 255u8, 255));

    let original = img.to_rgba_vec();
    sobel(SobelDirection::Horizontal)
      .with_options(ApplyOptions::new().with_area(Area::rect((2.0, 2.0), (4.0, 4.0))))
      .apply(&mut img);

    for y in 0..8u32 {
      for x in 0..8u32 {
        let idx = ((y * 8 + x) * 4) as usize;
        if x < 2 || x >= 6 || y < 2 || y >= 6 {
          assert_eq!(img.rgba()[idx], original[idx]);
          assert_eq!(img.rgba()[idx + 1], original[idx + 1]);
          assert_eq!(img.rgba()[idx + 2], original[idx + 2]);
          assert_eq!(img.rgba()[idx + 3], original[idx + 3]);
        }
      }
    }
  }
}
