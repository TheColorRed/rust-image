use abra_core::Image;
use options::{Effect, Options};
use rayon::prelude::*;

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

impl Effect for Posterize {
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
    apply_posterize(p_image, self.levels);
  }

  /// It changes each pixel on its own, so for a picture of any size it is the same.
  fn for_level(&self, _p_level: u32) -> Option<std::sync::Arc<dyn abra_core::image::gpu::CpuProcessor>> {
    Some(std::sync::Arc::new(self.clone()))
  }
}

pub fn posterize(p_levels: u8) -> Posterize {
  Posterize {
    levels: p_levels,
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::image::image_ext::ImageExt;
  use abra_core::{Channels, Image};

  #[test]
  fn a_part_of_a_deferred_posterize_at_any_zoom_is_the_posterize_of_the_picture_there() {
    let pixels = (0..48u32 * 32).flat_map(|at| [(at * 7) as u8, (at / 3) as u8, (at * 13) as u8, 255]).collect::<Vec<u8>>();
    let image = Image::new_from_pixels(48, 32, pixels, Channels::RGBA);
    let mut later = image.clone();
    posterize(4).apply(options::deferred(&mut later));
    assert!(later.deferred().is_some());
    let mut whole = image.clone();
    posterize(4).apply_on_cpu(&mut whole);
    assert_eq!(later.region(0, 5, 3, 20, 10), whole.region(0, 5, 3, 20, 10));
    // Halved twice, each pixel is still posterized on its own.
    let mut quarter = Image::new_from_pixels(12, 8, image.region(2, 0, 0, 12, 8), Channels::RGBA);
    posterize(4).apply_on_cpu(&mut quarter);
    assert_eq!(later.region(2, 2, 1, 6, 4), quarter.region(0, 2, 1, 6, 4));
  }
}
