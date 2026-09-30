use abra_core::{Image, ImageRef};
use options::{Apply, Options};
use rayon::prelude::*;

use crate::apply_adjustment;

fn apply_posterize(p_image: &mut Image, p_levels: u8) {
  let p_levels = (p_levels as f32).clamp(2.0, 255.0);
  let pixels = p_image.colors().as_slice_mut().expect("Image colors must be contiguous");

  pixels.par_chunks_mut(4).for_each(|pixel| {
    pixel[0] = ((pixel[0] as f32 / 255.0 * (p_levels - 1.0) as f32).round() / (p_levels - 1.0) as f32 * 255.0) as u8;
    pixel[1] = ((pixel[1] as f32 / 255.0 * (p_levels - 1.0) as f32).round() / (p_levels - 1.0) as f32 * 255.0) as u8;
    pixel[2] = ((pixel[2] as f32 / 255.0 * (p_levels - 1.0) as f32).round() / (p_levels - 1.0) as f32 * 255.0) as u8;
  });
}

#[derive(Clone)]
pub struct Posterize {
  levels: u8,
  options: Options,
}

options::cpu_processor!(Posterize);

impl Apply for Posterize {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(apply_posterize, image, self.options.as_ref(), 0, self.levels);
  }
}

pub fn posterize(p_levels: u8) -> Posterize {
  Posterize {
    levels: p_levels,
    options: None,
  }
}
