use crate::kernel::apply_kernel;
use core::Image;
// rayon is not used directly here; processing is delegated to `apply_kernel`, which performs parallelism when appropriate.
use core::image::apply_area::apply_processing;
use options::ApplyOptions;

/// Smooths the image using a 3x3 box blur kernel.
/// This version supports `ApplyOptions` to restrict and feather the operation.
pub fn smooth(image: &mut Image, options: impl Into<Option<ApplyOptions>>) {
  let options = options.into(); // Ensure options variable is used
  apply_processing(image, options.as_ref(), 1, |img| {
    let kernel = [0.0; 9].iter().map(|_| 1.0 / 9.0).collect::<Vec<f32>>();
    apply_kernel(img, kernel.as_slice());
  });
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::{Area, Image};
  use options::ApplyOptions;

  #[test]
  fn smooth_area_writes_back_only_area() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let orig = img.to_rgba_vec();

    smooth(&mut img, ApplyOptions::new().with_area(Area::rect((2.0, 2.0), (4.0, 4.0))));

    // Ensure outside area unchanged
    for y in 0..8u32 {
      for x in 0..8u32 {
        let idx = ((y * 8 + x) * 4) as usize;
        if x < 2 || x >= 6 || y < 2 || y >= 6 {
          assert_eq!(img.rgba()[idx], orig[idx]);
          assert_eq!(img.rgba()[idx + 1], orig[idx + 1]);
          assert_eq!(img.rgba()[idx + 2], orig[idx + 2]);
          assert_eq!(img.rgba()[idx + 3], orig[idx + 3]);
        }
      }
    }
  }

  #[test]
  fn smooth_changes_pixels_full_image_none_options() {
    let mut img = Image::new(8, 8);
    for y in 0..8u32 {
      for x in 0..8u32 {
        img.set_pixel(x, y, (0u8, 0u8, 0u8, 255));
      }
    }
    img.set_pixel(3, 3, (255u8, 0u8, 0u8, 255));
    let orig = img.to_rgba_vec();
    smooth(&mut img, None::<ApplyOptions>);
    // Expect center to have changed
    let idx = ((3 * 8 + 3) * 4) as usize;
    assert!(
      img.rgba()[idx] != orig[idx] || img.rgba()[idx + 1] != orig[idx + 1] || img.rgba()[idx + 2] != orig[idx + 2]
    );
  }
}
