use crate::kernel::apply_kernel;
use abra_core::Image;
use options::{Effect, Options};

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
