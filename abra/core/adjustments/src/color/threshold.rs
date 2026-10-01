use abra_core::{
  Image,
  image::gpu::{GpuPass, GpuProcessor},
};
use options::{Effect, Options};
use rayon::prelude::*;

/// Apply a threshold to an image where all pixels above the threshold are set to white and all pixels below are set to black.
/// * `p_image` - A mutable reference to the image to be processed.
/// * `threshold` - The threshold value a value between 0 and 255.
fn apply_threshold(p_image: &mut Image, p_threshold: u8) {
  let threshold = p_threshold.clamp(0, 255);
  let pixels = p_image.colors().as_slice_mut().expect("Image colors must be contiguous");

  pixels.par_chunks_mut(4).for_each(|pixel| {
    let avg = (pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32) / 3.0;

    if avg > threshold as f32 {
      pixel[0] = 255;
      pixel[1] = 255;
      pixel[2] = 255;
    } else {
      pixel[0] = 0;
      pixel[1] = 0;
      pixel[2] = 0;
    }
  });
}

#[derive(Clone)]
pub struct Threshold {
  threshold: u8,
  options: Options,
}

impl Effect for Threshold {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_threshold(p_image, self.threshold);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Threshold {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(
      include_str!("threshold.wgsl"),
      (self.threshold as f32).to_le_bytes().to_vec(),
    )]
  }
}

pub fn threshold(p_threshold: u8) -> Threshold {
  Threshold {
    threshold: p_threshold,
    options: None,
  }
}
