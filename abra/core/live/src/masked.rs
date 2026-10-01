//! Limits an effect to the areas and mask it was given.

use abra_core::Image;
use abra_core::image::apply_area::{mix_by_weights, weight_to_byte};
use abra_core::image::gpu::{GpuAux, GpuEffect, GpuPass, LiveEffect};
use std::sync::Arc;

const BLEND_SHADER: &str = include_str!("masked.wgsl");

/// An effect that only takes hold where its weights say so: `0` leaves a pixel as it was, `1` takes the effect's
/// result, and values between mix the two. This is how the CPU path applies an effect's area and mask too, so a live
/// image shows what the same edit would give when applied to the image.
///
/// The inner effect runs over the whole image and the result is mixed with the input afterwards, which gives the same
/// pixels as running it over the area alone, kernel effects included.
pub(crate) struct MaskedEffect {
  inner: Arc<dyn LiveEffect>,
  weights: GpuAux,
}

/// The weights as a texture: one weight from `0.0` to `1.0` per pixel of a `p_width` x `p_height` image, in the red
/// channel. Cloning it shares the pixels, so an effect that keeps its area and mask across many edits builds this
/// once.
pub(crate) fn weights_texture(p_width: u32, p_height: u32, p_weights: &[f32]) -> GpuAux {
  let rgba: Vec<u8> = p_weights
    .iter()
    .flat_map(|weight| {
      let byte = weight_to_byte(*weight);
      [byte, byte, byte, 255]
    })
    .collect();
  GpuAux {
    width: p_width,
    height: p_height,
    rgba: rgba.into(),
  }
}

impl MaskedEffect {
  /// Wraps `p_inner` so it only takes hold where `p_weights` (see [`weights_texture`]) say so.
  pub(crate) fn with_weights(p_inner: Arc<dyn LiveEffect>, p_weights: GpuAux) -> MaskedEffect {
    MaskedEffect {
      inner: p_inner,
      weights: p_weights,
    }
  }

  /// Wraps `p_inner` with one weight from `0.0` to `1.0` per pixel of a `p_width` x `p_height` image.
  #[cfg(test)]
  pub(crate) fn new(p_inner: Arc<dyn LiveEffect>, p_width: u32, p_height: u32, p_weights: &[f32]) -> MaskedEffect {
    MaskedEffect::with_weights(p_inner, weights_texture(p_width, p_height, p_weights))
  }
}

impl GpuEffect for MaskedEffect {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let mut passes = self.inner.passes(p_width, p_height);
    if !passes.is_empty() {
      passes.push(GpuPass::new(BLEND_SHADER, Vec::new()).with_aux(self.weights.clone()).with_base());
    }
    passes
  }
}

impl LiveEffect for MaskedEffect {
  fn has_gpu(&self) -> bool {
    self.inner.has_gpu()
  }

  fn apply_cpu(&self, p_image: &mut Image) {
    let before = p_image.to_rgba_vec();
    self.inner.apply_cpu(p_image);
    let (width, height) = p_image.dimensions::<u32>();
    if (width, height) != (self.weights.width, self.weights.height) {
      return;
    }
    let weights = self.weights.rgba.chunks_exact(4).map(|texel| texel[0]);
    p_image.set_rgba(mix_by_weights(&before, &p_image.to_rgba_vec(), weights));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  /// No shader: only the CPU implementation exists.
  struct Invert;

  impl GpuEffect for Invert {
    fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
      Vec::new()
    }
  }

  impl LiveEffect for Invert {
    fn has_gpu(&self) -> bool {
      false
    }

    fn apply_cpu(&self, p_image: &mut Image) {
      let pixels: Vec<u8> =
        p_image.to_rgba_vec().chunks_exact(4).flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]]).collect();
      p_image.set_rgba(pixels);
    }
  }

  #[test]
  fn a_cpu_effect_only_takes_hold_where_the_weights_say_so() {
    let mut image = Image::new_from_pixels(4, 1, [100, 100, 100, 255].repeat(4), Channels::RGBA);
    let masked = MaskedEffect::new(Arc::new(Invert), 4, 1, &[0.0, 0.5, 1.0, 0.0]);
    assert!(!masked.has_gpu());
    assert!(masked.passes(4, 1).is_empty(), "no shader means nothing to run on the GPU");
    masked.apply_cpu(&mut image);
    let pixels = image.to_rgba_vec();
    assert_eq!(pixels[0], 100, "weight 0 leaves the pixel alone");
    assert!(pixels[4].abs_diff(128) <= 1, "weight 0.5 mixes the input and the result: {}", pixels[4]);
    assert_eq!(pixels[8], 155, "weight 1 takes the result");
    assert_eq!(pixels[12], 100);
  }
}
