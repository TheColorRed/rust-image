use crate::common::*;

#[derive(Clone, Copy, Debug)]
/// Noise distribution modes for post-blur dithering
pub enum NoiseDistribution {
  /// Even probability across range
  Uniform,
  /// Normal distribution via Box-Muller
  Gaussian,
}

fn hash3(p_u: u32, p_v: u32, p_w: u32) -> u32 {
  // A simple integer hash (Thomas Wang mix)
  let mut x = p_u.wrapping_mul(374761393) ^ p_v.wrapping_mul(668265263) ^ p_w.wrapping_mul(2246822519);
  x ^= x >> 13;
  x = x.wrapping_mul(1274126177);
  x ^ (x >> 16)
}

fn rand01(p_seed: u32) -> f32 {
  (p_seed as f32) / (u32::MAX as f32)
}

fn gaussian_from_uniform(p_u1: f32, p_u2: f32) -> f32 {
  // Standard normal via Box-Muller (one sample)
  let r = (-2.0 * p_u1.max(1e-7).ln()).sqrt();
  let theta = 2.0 * std::f32::consts::PI * p_u2;
  r * theta.cos()
}

fn apply_add_noise(p_image: &mut Image, p_amount: f32, p_distribution: NoiseDistribution) {
  let src = p_image.rgba();
  let (width, height) = p_image.dimensions::<usize>();
  let mut out = vec![0u8; width * height * 4];

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let x = (idx % width) as u32;
    let y = (idx / width) as u32;
    let seed1 = hash3(x, y, 0);
    let seed2 = hash3(x ^ 0x9E3779B9, y ^ 0x85EBCA6B, 0 ^ 0xC2B2AE35);
    let n = match p_distribution {
      NoiseDistribution::Uniform => (rand01(seed1) * 2.0 - 1.0) * p_amount,
      NoiseDistribution::Gaussian => gaussian_from_uniform(rand01(seed1), rand01(seed2)) * p_amount,
    };
    let noise_value = n * 3.0;
    dst_px[0] = (src[idx * 4] as f32 + noise_value).clamp(0.0, 255.0) as u8;
    dst_px[1] = (src[idx * 4 + 1] as f32 + noise_value).clamp(0.0, 255.0) as u8;
    dst_px[2] = (src[idx * 4 + 2] as f32 + noise_value).clamp(0.0, 255.0) as u8;
    dst_px[3] = src[idx * 4 + 3];
  });
  p_image.set_rgba_owned(out);
}

/// Adds random noise to the image. Create one with [`noise`].
pub struct Noise {
  amount: f32,
  distribution: NoiseDistribution,
  options: Options,
}

impl Apply for Noise {
  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    let options = self.options.clone();
    apply_filter!(apply_add_noise, image, options, 1, self.amount, self.distribution);
  }
}

impl Noise {
  /// Sets how the noise values are distributed. Defaults to [`NoiseDistribution::Uniform`].
  pub fn with_distribution(mut self, p_distribution: NoiseDistribution) -> Self {
    self.distribution = p_distribution;
    self
  }
}

/// Adds random noise to the image.
/// # Arguments
/// - `p_amount`: The strength of the noise.
pub fn noise(p_amount: f32) -> Noise {
  Noise {
    amount: p_amount,
    distribution: NoiseDistribution::Uniform,
    options: None,
  }
}
