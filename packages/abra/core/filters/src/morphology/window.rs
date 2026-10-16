use crate::common::*;
use abra_core::IntoNumber;
use abra_core::image::gpu::{CpuProcessor, GpuPass, GpuProcessor};
use abra_core::image::levels::scaled_to_level;
use mask::Mask;

/// Which way a [`Morphology`] moves the edges of the bright parts of an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MorphologyKind {
  /// Each pixel takes the strongest value near it, so bright parts grow.
  Dilate,
  /// Each pixel takes the weakest value near it, so bright parts shrink.
  Erode,
}

/// Grows or shrinks the bright parts of an image by a number of pixels, each of the red, green and blue channels on its own.
/// The alpha is left as it was. The window is a square, and the edges of the image are not eaten. Made by [`dilate`] and
/// [`erode`].
#[derive(Clone)]
pub struct Morphology {
  kind: MorphologyKind,
  radius: u32,
  options: Options,
}

impl Morphology {
  /// Which way this moves the edges.
  pub fn kind(&self) -> MorphologyKind {
    self.kind
  }

  /// How many pixels the edges move.
  pub fn radius(&self) -> u32 {
    self.radius
  }
}

/// Grows each bright part of an image by `p_radius` pixels.
/// - `p_radius`: How many pixels the edges move out. `0` changes nothing.
pub fn dilate(p_radius: impl IntoNumber) -> Morphology {
  Morphology { kind: MorphologyKind::Dilate, radius: p_radius.into(), options: None }
}

/// Shrinks each bright part of an image by `p_radius` pixels.
/// - `p_radius`: How many pixels the edges move in. `0` changes nothing.
pub fn erode(p_radius: impl IntoNumber) -> Morphology {
  Morphology { kind: MorphologyKind::Erode, radius: p_radius.into(), options: None }
}

impl GpuProcessor for Morphology {
  /// A pass along the rows and a pass down the columns, in whole numbers like the CPU, so the pixels are exactly the same.
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    if self.radius == 0 {
      return Vec::new();
    }
    let strongest = u32::from(self.kind == MorphologyKind::Dilate);
    let pass = |direction: u32| {
      let uniforms: Vec<u8> = [self.radius, direction, strongest, 0].iter().flat_map(|value| value.to_le_bytes()).collect();
      GpuPass::new(include_str!("./window.wgsl"), uniforms).with_window().with_reach(self.radius as f32)
    };
    vec![pass(0), pass(1)]
  }
}

impl Effect for Morphology {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    self.radius as i32
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    if self.radius == 0 {
      return;
    }
    // Each color channel is a mask, so the one implementation of growing and shrinking a mask does the work.
    let (width, height) = p_image.dimensions::<u32>();
    let pixels = p_image.rgba().to_vec();
    let mut out = pixels.clone();
    for channel in 0..3 {
      let values = pixels.chunks_exact(4).map(|pixel| pixel[channel]).collect();
      let mask = Mask::from_values(width, height, values);
      let moved = match self.kind {
        MorphologyKind::Dilate => mask.dilated(self.radius),
        MorphologyKind::Erode => mask.eroded(self.radius),
      };
      for (pixel, value) in out.chunks_exact_mut(4).zip(moved.values()) {
        pixel[channel] = *value;
      }
    }
    p_image.set_rgba(out);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }

  /// A window that is half as wide for a picture that is half as wide, rounded to a whole pixel.
  fn for_level(&self, p_level: u32) -> Option<std::sync::Arc<dyn CpuProcessor>> {
    let radius = scaled_to_level(self.radius as f32, p_level).round() as u32;
    Some(std::sync::Arc::new(Morphology { radius, ..self.clone() }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// An image of values that change from pixel to pixel in every channel, so a window that is off shows.
  fn test_pixels(p_width: u32, p_height: u32) -> Vec<u8> {
    (0..p_width * p_height)
      .flat_map(|i| {
        let x = i.wrapping_mul(2_654_435_761) >> 13;
        [(x % 251) as u8, ((x >> 3) % 241) as u8, ((x >> 5) % 239) as u8, 255]
      })
      .collect()
  }

  #[test]
  fn dilating_grows_a_bright_dot_and_leaves_the_alpha() {
    let mut image = Image::new(5, 5);
    for y in 0..5 {
      for x in 0..5 {
        image.set_pixel(x, y, (0, 0, 0, 255));
      }
    }
    image.set_pixel(2, 2, (255, 10, 0, 255));
    dilate(1).apply(&mut image);
    assert_eq!(image.get_pixel(1, 1), Some((255, 10, 0, 255)));
    assert_eq!(image.get_pixel(3, 3), Some((255, 10, 0, 255)));
    assert_eq!(image.get_pixel(0, 0), Some((0, 0, 0, 255)));
  }

  #[test]
  fn eroding_shrinks_a_bright_square() {
    let mut image = Image::new(7, 7);
    for y in 0..7 {
      for x in 0..7 {
        let inside = (1..6).contains(&x) && (1..6).contains(&y);
        image.set_pixel(x, y, if inside { (255, 255, 255, 255) } else { (0, 0, 0, 255) });
      }
    }
    erode(1).apply(&mut image);
    assert_eq!(image.get_pixel(1, 1), Some((0, 0, 0, 255)));
    assert_eq!(image.get_pixel(2, 2), Some((255, 255, 255, 255)));
  }

  #[test]
  fn a_radius_of_zero_changes_nothing_on_the_cpu_or_the_gpu() {
    let mut image = Image::new_from_pixels(3, 1, vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255], abra_core::Channels::RGBA);
    let before = image.to_rgba_vec();
    dilate(0).apply(&mut image);
    assert_eq!(image.to_rgba_vec(), before);
    assert!(dilate(0).passes(3, 1).is_empty());
  }

  #[test]
  fn the_gpu_matches_the_cpu_for_growing_and_shrinking() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let (width, height) = (41u32, 27u32);
    let pixels = test_pixels(width, height);
    for effect in [dilate as fn(u32) -> Morphology, erode] {
      for radius in [1u32, 2, 5, 12, 40] {
        let mut image = Image::new_from_pixels(width, height, pixels.clone(), abra_core::Channels::RGBA);
        effect(radius).apply_on_cpu(&mut image);
        let expected = image.to_rgba_vec();
        let actual = renderer.process(&[effect(radius).passes(width, height)], width, height, &pixels)?;
        assert_eq!(actual.len(), expected.len());
        for (index, (a, e)) in actual.iter().zip(&expected).enumerate() {
          assert_eq!(a, e, "{:?} radius {radius}, byte {index}: gpu {a} vs cpu {e}", effect(radius).kind());
        }
      }
    }
    Ok(())
  }
}
