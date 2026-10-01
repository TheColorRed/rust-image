use abra_core::{
  Channel, Image,
  image::gpu::{GpuOp, GpuPass, GpuProcessor},
};
use options::{Effect, Options};

fn apply_invert<'a>(p_image: &mut Image) {
  p_image.mut_channels(Channel::RGB, |channel| 255 - channel);
}

/// Inverts the colors of an image
#[derive(Default, Clone)]
pub struct Invert {
  options: Options,
}

impl Effect for Invert {
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
    apply_invert(p_image);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Invert {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<abra_core::image::gpu::GpuPass> {
    vec![GpuPass::new(include_str!("invert.wgsl"), Vec::new())]
  }
}

pub fn invert() -> Invert {
  Invert::default()
}
