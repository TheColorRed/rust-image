//! Shaders for effects where each output channel depends only on the same input channel.

use abra_core::image::gpu::{GpuAux, GpuPass};
use abra_core::{Channels, Image};

const CHANNEL_LUT_SHADER: &str = include_str!("./channel_lut.wgsl");

/// A GPU pass for an effect that changes each red, green and blue value on its own, such as brightness or contrast.
///
/// The pass is a 256-entry lookup table built by running `p_cpu` (the effect's own CPU code) over a gray ramp, so the
/// GPU gives exactly the pixels the CPU does, whatever arithmetic the effect uses. Every effect that uses this shares
/// one compiled shader.
/// - `p_cpu`: The effect's CPU code, run over an image with one pixel for each value from 0 to 255.
pub(crate) fn channel_lut_pass(p_cpu: impl FnOnce(&mut Image)) -> GpuPass {
  let ramp: Vec<u8> = (0..=255u8).flat_map(|value| [value, value, value, 255]).collect();
  let mut image = Image::new_from_pixels(256, 1, ramp, Channels::RGBA);
  p_cpu(&mut image);
  GpuPass::new(CHANNEL_LUT_SHADER, Vec::new()).with_aux(GpuAux {
    width: 256,
    height: 1,
    rgba: image.to_rgba_vec().into(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_table_is_what_the_cpu_code_does_to_each_value() {
    let pass = channel_lut_pass(|image| {
      let doubled: Vec<u8> = image.to_rgba_vec().chunks_exact(4).flat_map(|p| [p[0].saturating_mul(2), p[1], p[2], p[3]]).collect();
      image.set_rgba(doubled);
    });
    let table = pass.aux.expect("a table").rgba;
    assert_eq!(table.len(), 256 * 4);
    assert_eq!(table[100 * 4], 200);
    assert_eq!(table[200 * 4], 255);
    assert_eq!(table[100 * 4 + 1], 100, "the other channels keep their own entry");
  }
}
