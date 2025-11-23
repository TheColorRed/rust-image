use rayon::prelude::*;

use core::Image;

#[derive(Clone, Copy, Debug)]
/// The aperture shape for the lens blur iris (number of blades)
pub enum ApertureShape {
  /// Three-blade iris (triangle bokeh)
  Triangle,
  /// Four-blade iris (square bokeh)
  Square,
  /// Five-blade iris (pentagon bokeh)
  Pentagon,
  /// Six-blade iris (hexagon bokeh)
  Hexagon,
  /// Seven-blade iris (heptagon bokeh)
  Heptagon,
  /// Eight-blade iris (octagon bokeh)
  Octagon,
}

impl ApertureShape {
  fn blades(&self) -> u32 {
    match self {
      ApertureShape::Triangle => 3,
      ApertureShape::Square => 4,
      ApertureShape::Pentagon => 5,
      ApertureShape::Hexagon => 6,
      ApertureShape::Heptagon => 7,
      ApertureShape::Octagon => 8,
    }
  }
}

#[derive(Clone, Copy, Debug)]
/// Noise distribution modes for post-blur dithering
pub enum NoiseDistribution {
  /// Even probability across range
  Uniform,
  /// Normal distribution via Box-Muller
  Gaussian,
}

#[derive(Clone, Copy, Debug)]
/// Iris configuration
pub struct IrisOptions {
  /// Aperture polygon shape: triangle through octagon
  pub shape: ApertureShape,
  /// Radius in pixels of the blur kernel
  pub radius: u32,
  /// Blade curvature (0.0 polygon, 1.0 circle)
  pub blade_curvature: f32,
  /// Rotation of the aperture in radians
  pub rotation: f32,
}

#[derive(Clone, Copy, Debug)]
/// Specular highlight configuration
pub struct SpecularOptions {
  /// Multiplier applied to samples above threshold (>= 1.0)
  pub brightness: f32,
  /// Luminance threshold in [0.0, 1.0]
  pub threshold: f32,
}

#[derive(Clone, Copy, Debug)]
/// Noise configuration
pub struct NoiseOptions {
  /// Strength of noise in [0.0, 1.0] relative to 255 range
  pub amount: f32,
  /// Noise distribution
  pub distribution: NoiseDistribution,
}

#[derive(Clone, Copy, Debug)]
/// Options for lens blur filter
pub struct LensBlurOptions {
  /// Iris (aperture) configuration
  pub iris: IrisOptions,
  /// Specular highlight boost; None to disable
  pub specular: Option<SpecularOptions>,
  /// Output noise/dither; None to disable
  pub noise: Option<NoiseOptions>,
  /// Number of samples per pixel. Higher is smoother but slower.
  pub samples: u32,
}

impl Default for IrisOptions {
  fn default() -> Self {
    Self {
      shape: ApertureShape::Hexagon,
      radius: 8,
      blade_curvature: 0.5,
      rotation: 0.0,
    }
  }
}

impl Default for LensBlurOptions {
  fn default() -> Self {
    Self {
      iris: IrisOptions::default(),
      specular: None,
      noise: None,
      samples: 32,
    }
  }
}

#[inline]
fn luminance_rgb(r: f32, g: f32, b: f32) -> f32 {
  0.2126 * r + 0.7152 * g + 0.0722 * b
}

// Regular polygon radius for given angle theta.
// N: number of sides, R: circumradius.
#[inline]
fn polygon_radius(theta: f32, blades: u32, r: f32) -> f32 {
  let n = blades as f32;
  let k = (std::f32::consts::PI / n).cos();
  let a = (theta % (2.0 * std::f32::consts::PI / n)) - (std::f32::consts::PI / n);
  (k * r) / a.cos()
}

// Boundary radius blending from polygon to circle based on blade_curvature in [0,1].
#[inline]
fn iris_boundary(theta: f32, iris: &IrisOptions) -> f32 {
  let r = iris.radius as f32;
  let rp = polygon_radius(theta, iris.shape.blades(), r);
  let rc = r;
  rp * (1.0 - iris.blade_curvature) + rc * iris.blade_curvature
}

// Generate a low-discrepancy sample in [0,1]^2 using Vogel spiral approximation
#[inline]
fn ld_sample(i: u32, n: u32) -> (f32, f32) {
  // Golden angle ratio sequence
  let g = 0.61803398875_f32; // (sqrt(5)-1)/2
  let u = (i as f32 + 0.5) / n as f32;
  let v = ((i as f32) * g).fract();
  (u, v)
}

// Map unit square sample to the iris shape area-uniformly.
#[inline]
fn iris_sample_offset(i: u32, n: u32, iris: &IrisOptions) -> (f32, f32) {
  let (u, v) = ld_sample(i, n);
  let r_unit = u.sqrt(); // area-uniform radius in [0,1]
  let mut theta = 2.0 * std::f32::consts::PI * v;
  theta += iris.rotation;
  let r_boundary = iris_boundary(theta, iris);
  let r = r_unit * r_boundary;
  let (dx, dy) = (r * theta.cos(), r * theta.sin());
  (dx, dy)
}

#[inline]
fn hash3(u: u32, v: u32, w: u32) -> u32 {
  // A simple integer hash (Thomas Wang mix)
  let mut x = u.wrapping_mul(374761393) ^ v.wrapping_mul(668265263) ^ w.wrapping_mul(2246822519);
  x ^= x >> 13;
  x = x.wrapping_mul(1274126177);
  x ^ (x >> 16)
}

#[inline]
fn rand01(seed: u32) -> f32 {
  (seed as f32) / (u32::MAX as f32)
}

#[inline]
fn gaussian_from_uniform(u1: f32, u2: f32) -> f32 {
  // Standard normal via Box-Muller (one sample)
  let r = (-2.0 * u1.max(1e-7).ln()).sqrt();
  let theta = 2.0 * std::f32::consts::PI * u2;
  r * theta.cos()
}

#[inline]
fn add_noise(rgb: &mut [f32; 3], x: u32, y: u32, c: u32, opt: &NoiseOptions) {
  let seed1 = hash3(x, y, c);
  let seed2 = hash3(x ^ 0x9E3779B9, y ^ 0x85EBCA6B, c ^ 0xC2B2AE35);
  let n = match opt.distribution {
    NoiseDistribution::Uniform => (rand01(seed1) * 2.0 - 1.0) * opt.amount,
    NoiseDistribution::Gaussian => gaussian_from_uniform(rand01(seed1), rand01(seed2)) * (opt.amount * 0.5),
  };
  let scale = 255.0;
  rgb[0] = (rgb[0] + n * scale).clamp(0.0, 255.0);
  rgb[1] = (rgb[1] + n * scale).clamp(0.0, 255.0);
  rgb[2] = (rgb[2] + n * scale).clamp(0.0, 255.0);
}

/// Applies a lens blur to an image with polygonal/circular iris, specular highlights and optional noise.
/// - `image`: target image buffer
/// - `p_options`: lens blur configuration
pub fn lens_blur(image: &mut Image, p_options: LensBlurOptions) {
  let samples = p_options.samples.max(1);
  let (width, height) = image.dimensions::<u32>();
  if p_options.iris.radius == 0 || width == 0 || height == 0 {
    return;
  }

  // Precompute offsets
  let offsets: Vec<(f32, f32)> = (0..samples)
    .map(|i| iris_sample_offset(i, samples, &p_options.iris))
    .collect();

  // Snapshot source pixels once (borrow slice to avoid copying)
  let src = image.rgba_slice();
  let (w, h) = (width as usize, height as usize);

  let mut out = vec![0u8; w * h * 4];

  out.par_chunks_mut(4).enumerate().for_each(|(idx, dst_px)| {
    let x = (idx % w) as u32;
    let y = (idx / w) as u32;

    let mut acc_r = 0.0f32;
    let mut acc_g = 0.0f32;
    let mut acc_b = 0.0f32;
    let mut acc_a = 0.0f32;

    for (dx, dy) in &offsets {
      let fx = x as f32 + *dx;
      let fy = y as f32 + *dy;

      // Bilinear sample from source snapshot
      let (mut r, mut g, mut b, a) = {
        // Manual bilinear from src snapshot without Image borrow complications
        let (wi, hi) = (width as i32, height as i32);
        let sx = fx.clamp(0.0, (wi - 1) as f32);
        let sy = fy.clamp(0.0, (hi - 1) as f32);
        let x0 = sx.floor() as i32;
        let y0 = sy.floor() as i32;
        let x1 = (x0 + 1).min(wi - 1);
        let y1 = (y0 + 1).min(hi - 1);
        let tx = sx - x0 as f32;
        let ty = sy - y0 as f32;

        let i00 = ((y0 as usize) * w + x0 as usize) * 4;
        let i10 = ((y0 as usize) * w + x1 as usize) * 4;
        let i01 = ((y1 as usize) * w + x0 as usize) * 4;
        let i11 = ((y1 as usize) * w + x1 as usize) * 4;

        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

        let r0 = lerp(src[i00] as f32, src[i10] as f32, tx);
        let g0 = lerp(src[i00 + 1] as f32, src[i10 + 1] as f32, tx);
        let b0 = lerp(src[i00 + 2] as f32, src[i10 + 2] as f32, tx);
        let a0 = lerp(src[i00 + 3] as f32, src[i10 + 3] as f32, tx);

        let r1 = lerp(src[i01] as f32, src[i11] as f32, tx);
        let g1 = lerp(src[i01 + 1] as f32, src[i11 + 1] as f32, tx);
        let b1 = lerp(src[i01 + 2] as f32, src[i11 + 2] as f32, tx);
        let a1 = lerp(src[i01 + 3] as f32, src[i11 + 3] as f32, tx);

        (lerp(r0, r1, ty), lerp(g0, g1, ty), lerp(b0, b1, ty), lerp(a0, a1, ty))
      };

      if let Some(spec) = p_options.specular {
        let lum = luminance_rgb(r, g, b) / 255.0;
        if lum > spec.threshold {
          let m = spec.brightness.max(1.0);
          r *= m;
          g *= m;
          b *= m;
        }
      }

      acc_r += r;
      acc_g += g;
      acc_b += b;
      acc_a += a;
    }

    let inv = 1.0 / samples as f32;
    let mut rgb = [acc_r * inv, acc_g * inv, acc_b * inv];
    let a = (acc_a * inv).clamp(0.0, 255.0);

    if let Some(noise) = p_options.noise {
      if noise.amount > 0.0 {
        add_noise(&mut rgb, x, y, 0, &noise);
      }
    }

    dst_px[0] = rgb[0].clamp(0.0, 255.0) as u8;
    dst_px[1] = rgb[1].clamp(0.0, 255.0) as u8;
    dst_px[2] = rgb[2].clamp(0.0, 255.0) as u8;
    dst_px[3] = a as u8;
  });

  image.set_rgba(out);
}
