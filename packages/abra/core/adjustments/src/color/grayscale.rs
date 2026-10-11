use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};

/// Converts an image to grayscale
fn apply_grayscale(p_image_ref: &mut Image) {
  p_image_ref.mut_pixels(|mut pixel| {
    let r = pixel[0] as f32;
    let g = pixel[1] as f32;
    let b = pixel[2] as f32;

    // Map the pixel to a grayscale value.
    let gray = ((77.0 * r + 150.0 * g + 29.0 * b + 128.0) / 256.0) as u8; // r, g, b as f32

    // Set the pixel to the grayscale value.
    pixel[0] = gray;
    pixel[1] = gray;
    pixel[2] = gray;
  });
}

#[derive(Default, Clone)]
pub struct Grayscale {
  options: Options,
}

impl Effect for Grayscale {
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
    apply_grayscale(p_image);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Grayscale {
  fn passes(&self, _w: u32, _h: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./grayscale.wgsl"), Vec::new())]
  }
}

pub fn grayscale() -> Grayscale {
  Grayscale::default()
}
