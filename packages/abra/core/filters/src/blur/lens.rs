use crate::common::*;
use abra_core::IntoNumber;

use crate::noise::NoiseDistribution;

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

/// Iris (aperture) settings.
#[derive(Clone, Copy, Debug)]
struct IrisSettings {
  /// Aperture polygon shape: triangle through octagon
  shape: ApertureShape,
  /// Radius in pixels of the blur kernel
  radius: u32,
  /// Blade curvature (0.0 polygon, 1.0 circle)
  blade_curvature: f32,
  /// Rotation of the aperture in radians
  rotation: f32,
}

/// Specular highlight settings.
#[derive(Clone, Copy, Debug)]
struct SpecularSettings {
  /// Multiplier applied to samples above threshold (>= 1.0)
  brightness: f32,
  /// Luminance threshold in [0.0, 1.0]
  threshold: f32,
}

/// Output noise settings.
#[derive(Clone, Copy, Debug)]
struct NoiseSettings {
  /// Strength of noise in [0.0, 1.0] relative to 255 range
  amount: f32,
  /// Noise distribution
  distribution: NoiseDistribution,
}

/// Everything the lens blur needs, gathered so it can be handed to the pixel loop in one piece.
#[derive(Clone, Copy, Debug)]
struct LensSettings {
  iris: IrisSettings,
  /// Specular highlight boost; None to disable
  specular: Option<SpecularSettings>,
  /// Output noise/dither; None to disable
  noise: Option<NoiseSettings>,
  /// Number of samples per pixel. Higher is smoother but slower.
  samples: u32,
}

#[inline]
fn luminance_rgb(p_r: f32, p_g: f32, p_b: f32) -> f32 {
  0.2126 * p_r + 0.7152 * p_g + 0.0722 * p_b
}

// Regular polygon radius for given angle theta.
// N: number of sides, R: circumradius.
#[inline]
fn polygon_radius(p_theta: f32, p_blades: u32, p_r: f32) -> f32 {
  let n = p_blades as f32;
  let k = (std::f32::consts::PI / n).cos();
  let a = (p_theta % (2.0 * std::f32::consts::PI / n)) - (std::f32::consts::PI / n);
  (k * p_r) / a.cos()
}

// Boundary radius blending from polygon to circle based on blade_curvature in [0,1].
#[inline]
fn iris_boundary(p_theta: f32, p_iris: &IrisSettings) -> f32 {
  let r = p_iris.radius as f32;
  let rp = polygon_radius(p_theta, p_iris.shape.blades(), r);
  let rc = r;
  rp * (1.0 - p_iris.blade_curvature) + rc * p_iris.blade_curvature
}

// Generate a low-discrepancy sample in [0,1]^2 using Vogel spiral approximation
#[inline]
fn ld_sample(p_i: u32, p_n: u32) -> (f32, f32) {
  // Golden angle ratio sequence
  let g = 0.61803398875_f32; // (sqrt(5)-1)/2
  let u = (p_i as f32 + 0.5) / p_n as f32;
  let v = ((p_i as f32) * g).fract();
  (u, v)
}

// Map unit square sample to the iris shape area-uniformly.
#[inline]
fn iris_sample_offset(p_i: u32, p_n: u32, p_iris: &IrisSettings) -> (f32, f32) {
  let (u, v) = ld_sample(p_i, p_n);
  let r_unit = u.sqrt(); // area-uniform radius in [0,1]
  let mut theta = 2.0 * std::f32::consts::PI * v;
  theta += p_iris.rotation;
  let r_boundary = iris_boundary(theta, p_iris);
  let r = r_unit * r_boundary;
  let (dx, dy) = (r * theta.cos(), r * theta.sin());
  (dx, dy)
}

/// Applies a lens blur to an image with polygonal/circular iris, specular highlights and optional noise.
/// - `image`: target image buffer
/// - `p_options`: lens blur configuration
fn apply_lens_blur(p_image: &mut Image, p_options: LensSettings) {
  let samples = p_options.samples.max(1);
  let (width, height) = p_image.dimensions::<u32>();
  if p_options.iris.radius == 0 || width == 0 || height == 0 {
    return;
  }

  // Precompute offsets
  let offsets: Vec<(f32, f32)> = (0..samples).map(|i| iris_sample_offset(i, samples, &p_options.iris)).collect();

  // Snapshot source pixels once (borrow slice to avoid copying)
  let src = p_image.rgba();
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
    let rgb = [acc_r * inv, acc_g * inv, acc_b * inv];
    let a = (acc_a * inv).clamp(0.0, 255.0);

    dst_px[0] = rgb[0].clamp(0.0, 255.0) as u8;
    dst_px[1] = rgb[1].clamp(0.0, 255.0) as u8;
    dst_px[2] = rgb[2].clamp(0.0, 255.0) as u8;
    dst_px[3] = a as u8;
  });

  p_image.set_rgba(out);

  if let Some(noise) = p_options.noise {
    if noise.amount > 0.0 {
      crate::noise::noise(noise.amount).with_distribution(noise.distribution).apply(p_image);
    }
  }
}
/// A lens blur. Create one with [`lens_blur`], adjust it with the `with_*` methods, then run it with
/// [`Apply::apply`].
#[derive(Clone)]
pub struct LensBlur {
  settings: LensSettings,
  options: Options,
}

impl LensBlur {
  /// Sets the aperture shape, which shapes the bokeh. Defaults to [`ApertureShape::Hexagon`].
  pub fn with_shape(mut self, p_shape: ApertureShape) -> Self {
    self.settings.iris.shape = p_shape;
    self
  }

  /// Sets how round the aperture blades are, from `0.0` (straight polygon) to `1.0` (circle). Defaults to `0.5`.
  pub fn with_blade_curvature(mut self, p_curvature: f32) -> Self {
    self.settings.iris.blade_curvature = p_curvature.clamp(0.0, 1.0);
    self
  }

  /// Sets the clockwise rotation of the aperture in degrees. Defaults to `0.0`.
  pub fn with_rotation(mut self, p_degrees: f32) -> Self {
    self.settings.iris.rotation = p_degrees.to_radians();
    self
  }

  /// Brightens highlights to make bokeh stand out. Samples brighter than `p_threshold` (luminance, `0.0..=1.0`)
  /// are multiplied by `p_brightness` (at least `1.0`). Off by default.
  pub fn with_specular(mut self, p_brightness: f32, p_threshold: f32) -> Self {
    self.settings.specular = Some(SpecularSettings {
      brightness: p_brightness.max(1.0),
      threshold: p_threshold.clamp(0.0, 1.0),
    });
    self
  }

  /// Adds noise after blurring, to match the grain of the rest of the image. `p_amount` is `0.0..=1.0`.
  /// Off by default.
  pub fn with_noise(mut self, p_amount: f32, p_distribution: NoiseDistribution) -> Self {
    self.settings.noise = Some(NoiseSettings {
      amount: p_amount.clamp(0.0, 1.0),
      distribution: p_distribution,
    });
    self
  }

  /// Sets the number of samples per pixel. Higher is smoother but slower. Defaults to `32`.
  pub fn with_samples(mut self, p_samples: u32) -> Self {
    self.settings.samples = p_samples.max(1);
    self
  }
}

impl Effect for LensBlur {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    self.settings.iris.radius as i32 + 1
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    apply_lens_blur(p_image, self.settings);
  }
}

/// Simulates a camera lens blur, where the aperture shape gives out-of-focus highlights their shape.
/// # Arguments
/// - `p_radius`: The radius of the blur in pixels.
///
/// By default the aperture is a hexagon with half-rounded blades, sampled 32 times per pixel, with no specular
/// boost or noise.
pub fn lens_blur(p_radius: impl IntoNumber) -> LensBlur {
  LensBlur {
    settings: LensSettings {
      iris: IrisSettings {
        shape: ApertureShape::Hexagon,
        radius: p_radius.into(),
        blade_curvature: 0.5,
        rotation: 0.0,
      },
      specular: None,
      noise: None,
      samples: 32,
    },
    options: None,
  }
}
