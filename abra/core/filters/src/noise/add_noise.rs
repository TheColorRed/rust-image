use crate::common::*;

#[derive(Clone, Copy, Debug)]
/// Noise distribution modes for post-blur dithering
pub enum NoiseDistribution {
  /// Even probability across range
  Uniform,
  /// Normal distribution via Box-Muller
  Gaussian,
}

fn hash3(u: u32, v: u32, w: u32) -> u32 {
  // A simple integer hash (Thomas Wang mix)
  let mut x = u.wrapping_mul(374761393) ^ v.wrapping_mul(668265263) ^ w.wrapping_mul(2246822519);
  x ^= x >> 13;
  x = x.wrapping_mul(1274126177);
  x ^ (x >> 16)
}

fn rand01(seed: u32) -> f32 {
  (seed as f32) / (u32::MAX as f32)
}

fn gaussian_from_uniform(u1: f32, u2: f32) -> f32 {
  // Standard normal via Box-Muller (one sample)
  let r = (-2.0 * u1.max(1e-7).ln()).sqrt();
  let theta = 2.0 * std::f32::consts::PI * u2;
  r * theta.cos()
}

fn apply_add_noise(p_image: &mut Image, amount: f32, distribution: NoiseDistribution) {
  let src = p_image.rgba();
  let (width, height) = p_image.dimensions::<usize>();
  let mut out = vec![0u8; width * height * 4];

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let x = (idx % width) as u32;
    let y = (idx / width) as u32;
    let seed1 = hash3(x, y, 0);
    let seed2 = hash3(x ^ 0x9E3779B9, y ^ 0x85EBCA6B, 0 ^ 0xC2B2AE35);
    let n = match distribution {
      NoiseDistribution::Uniform => (rand01(seed1) * 2.0 - 1.0) * amount,
      NoiseDistribution::Gaussian => gaussian_from_uniform(rand01(seed1), rand01(seed2)) * amount,
    };
    let noise_value = n * 3.0;
    dst_px[0] = (src[idx * 4] as f32 + noise_value).clamp(0.0, 255.0) as u8;
    dst_px[1] = (src[idx * 4 + 1] as f32 + noise_value).clamp(0.0, 255.0) as u8;
    dst_px[2] = (src[idx * 4 + 2] as f32 + noise_value).clamp(0.0, 255.0) as u8;
    dst_px[3] = src[idx * 4 + 3];
  });
  p_image.set_rgba_owned(out);
}

/// Applies a despeckle filter to the image, removing isolated noise pixels while preserving edges.
/// - `p_image`: The image to apply the filter to.
/// - `p_apply_options`: Options to specify for the filter.
pub fn noise<'a>(
  p_image: impl Into<ImageRef<'a>>, amount: f32, distribution: NoiseDistribution, p_apply_options: impl Into<Options>,
) {
  let mut image_ref: ImageRef = p_image.into();
  let image = &mut image_ref as &mut Image;
  apply_filter!(apply_add_noise, image, p_apply_options, 1, amount, distribution);
}
