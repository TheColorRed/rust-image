use crate::common::*;
use abra_core::Color;
use abra_core::image::gpu::{GpuPass, GpuProcessor};

use std::f32::consts::{FRAC_PI_4, PI};

/// The darkness at which a dot is as wide as its cell and touches its neighbors: the share of a square that a circle across it covers.
const FULL_DOT: f32 = FRAC_PI_4;

/// How dark a pixel is, from 0 for white to 1 for black, and 0 where it is clear.
fn darkness_of(p_pixel: &[u8]) -> f32 {
  let luma = (0.299 * p_pixel[0] as f32 + 0.587 * p_pixel[1] as f32 + 0.114 * p_pixel[2] as f32) / 255.0;
  (1.0 - luma) * (p_pixel[3] as f32 / 255.0)
}

fn apply_halftone(p_image: &mut Image, p_spacing: f32, p_angle: f32, p_samples: u32, p_ink: [f32; 3], p_paper: [f32; 3]) {
  let (width, height) = p_image.dimensions::<usize>();
  if width == 0 || height == 0 {
    return;
  }
  let source = p_image.rgba();
  let spacing = p_spacing.max(1.0);
  let (sin, cos) = p_angle.to_radians().sin_cos();
  let taps = p_samples.max(1);
  let mut out = vec![0u8; width * height * 4];
  out.par_chunks_mut(4).enumerate().for_each(|(index, pixel)| {
    let (x, y) = ((index % width) as f32 + 0.5, (index / width) as f32 + 0.5);
    // The place in the rows of dots: `u` along a row and `v` across the rows.
    let u = x * cos + y * sin;
    let v = -x * sin + y * cos;
    let (center_u, center_v) = ((u / spacing).round() * spacing, (v / spacing).round() * spacing);
    let center = (center_u * cos - center_v * sin, center_u * sin + center_v * cos);
    // How dark the cell is: the mean of a grid of pixels spread over it, so every pixel of a cell gets the same dot.
    let mut sum = 0.0;
    for a in 0..taps {
      for b in 0..taps {
        let du = ((a as f32 + 0.5) / taps as f32 - 0.5) * spacing;
        let dv = ((b as f32 + 0.5) / taps as f32 - 0.5) * spacing;
        let at_x = (center.0 + du * cos - dv * sin).floor().clamp(0.0, (width - 1) as f32) as usize;
        let at_y = (center.1 + du * sin + dv * cos).floor().clamp(0.0, (height - 1) as f32) as usize;
        sum += darkness_of(&source[(at_y * width + at_x) * 4..][..4]);
      }
    }
    let darkness = sum / (taps * taps) as f32;
    // The share of the cell that is ink is the share that is dark. Up to a darkness of a quarter of a circle's share of a square (where
    // the dots touch their neighbors) it is a dot of that area; beyond it the paper is what is left of the corners, four quarters of a
    // gap of the area that is not dark. Each is a little less than its area at the start of the other, so that none is left at the ends.
    let (across, along) = (u - center_u, v - center_v);
    let coverage = if darkness <= FULL_DOT {
      let radius = spacing * (darkness / PI).sqrt() - 0.5 * (1.0 - darkness);
      (radius - (across * across + along * along).sqrt() + 0.5).clamp(0.0, 1.0)
    } else {
      let gap = spacing * ((1.0 - darkness) / PI).sqrt() - 0.5 * darkness;
      let corner = ((spacing * 0.5 - across.abs()).powi(2) + (spacing * 0.5 - along.abs()).powi(2)).sqrt();
      1.0 - (gap - corner + 0.5).clamp(0.0, 1.0)
    };
    for channel in 0..3 {
      let value = p_paper[channel] + (p_ink[channel] - p_paper[channel]) * coverage;
      pixel[channel] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    pixel[3] = source[index * 4 + 3];
  });
  p_image.set_rgba(out);
}

/// Turns an image into a halftone screen, as a printed comic or newspaper is made: dots of one ink on a paper color, in rows turned by an
/// angle, where the share of the cell that is ink is the share that is dark under it. Light parts get small dots with paper between them,
/// and dark parts get dots that grow until they touch and then join, leaving rounded gaps of paper that close into solid ink. Create one with [`halftone`].
///
/// The darkness is how dark the pixels of a dot's cell are on average, so one dot has one size, and the size changes smoothly from one dot
/// to the next as the picture does. The ink and paper default to black and white, which gives the dots of Photoshop's Halftone Pattern and
/// can be multiplied over a flat color. The alpha of the image is kept, and a clear pixel counts as no darkness.
///
/// The rows of dots start from the corner of the image, so the dots do not move when the effect is limited to an area.
#[derive(Clone)]
pub struct Halftone {
  spacing: f32,
  angle: f32,
  samples: u32,
  ink: Color,
  paper: Color,
  options: Options,
}

impl Halftone {
  /// Turns the rows of dots by this many degrees, clockwise. Defaults to 45, as a printed screen is turned. 0 puts the dots in upright
  /// rows and columns.
  pub fn with_angle(mut self, p_degrees: f32) -> Self {
    self.angle = p_degrees;
    self
  }

  /// The color of the dots. Defaults to black. Its alpha is not used.
  pub fn with_ink(mut self, p_ink: Color) -> Self {
    self.ink = p_ink;
    self
  }

  /// The color between the dots. Defaults to white. Its alpha is not used.
  pub fn with_paper(mut self, p_paper: Color) -> Self {
    self.paper = p_paper;
    self
  }

  /// How many pixels in each direction are looked at over a dot's cell to work out how dark it is: more is slower and follows fine detail
  /// better. Defaults to 5.
  pub fn with_samples(mut self, p_samples: u32) -> Self {
    self.samples = p_samples;
    self
  }

  fn colors(&self) -> ([f32; 3], [f32; 3]) {
    let channels = |color: &Color| {
      let (red, green, blue) = color.rgb();
      [red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0]
    };
    (channels(&self.ink), channels(&self.paper))
  }
}

impl GpuProcessor for Halftone {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let (ink, paper) = self.colors();
    let (sin, cos) = self.angle.to_radians().sin_cos();
    let mut uniforms = Vec::with_capacity(48);
    uniforms.extend_from_slice(&self.spacing.max(1.0).to_le_bytes());
    uniforms.extend_from_slice(&cos.to_le_bytes());
    uniforms.extend_from_slice(&sin.to_le_bytes());
    uniforms.extend_from_slice(&self.samples.max(1).to_le_bytes());
    for value in [ink[0], ink[1], ink[2], 1.0, paper[0], paper[1], paper[2], 1.0] {
      uniforms.extend_from_slice(&value.to_le_bytes());
    }
    vec![GpuPass::new(include_str!("halftone.wgsl"), uniforms).with_reach(self.spacing.max(1.0) * 1.5)]
  }
}

impl Effect for Halftone {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// A dot's cell is looked at from a pixel up to a cell's width away.
  fn padding(&self) -> i32 {
    (self.spacing.max(1.0) * 1.5).ceil() as i32
  }

  /// The rows of dots start from the corner of the image.
  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let (ink, paper) = self.colors();
    apply_halftone(p_image, self.spacing, self.angle, self.samples, ink, paper);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

/// A halftone screen of the image, with this distance between the dots of a row.
/// - `p_spacing`: The distance in pixels between the centers of neighboring dots in a row. Less than 1 is taken as 1.
pub fn halftone(p_spacing: f32) -> Halftone {
  Halftone {
    spacing: p_spacing,
    angle: 45.0,
    samples: 5,
    ink: Color::black(),
    paper: Color::white(),
    options: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  /// A gray picture whose gray at each pixel is given.
  fn gray_image(p_width: u32, p_height: u32, p_gray: impl Fn(u32, u32) -> u8) -> Image {
    let mut pixels = Vec::with_capacity((p_width * p_height * 4) as usize);
    for y in 0..p_height {
      for x in 0..p_width {
        let gray = p_gray(x, y);
        pixels.extend_from_slice(&[gray, gray, gray, 255]);
      }
    }
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  /// The share of the pixels of an area that are ink, from the red channel of a black ink on white paper.
  fn ink_share(p_image: &Image, p_x: u32, p_y: u32, p_width: u32, p_height: u32) -> f32 {
    let (width, _) = p_image.dimensions::<u32>();
    let rgba = p_image.rgba();
    let mut sum = 0.0;
    for y in p_y..p_y + p_height {
      for x in p_x..p_x + p_width {
        sum += 1.0 - rgba[((y * width + x) * 4) as usize] as f32 / 255.0;
      }
    }
    sum / (p_width * p_height) as f32
  }

  #[test]
  fn white_is_all_paper_and_black_is_all_ink() {
    let mut white = gray_image(48, 48, |_, _| 255);
    halftone(8.0).apply_on_cpu(&mut white);
    assert!(ink_share(&white, 0, 0, 48, 48) < 0.001);
    let mut black = gray_image(48, 48, |_, _| 0);
    halftone(8.0).apply_on_cpu(&mut black);
    assert!(ink_share(&black, 0, 0, 48, 48) > 0.999);
  }

  #[test]
  fn the_darker_the_picture_the_more_ink_there_is() {
    // A ramp from black on the left to white on the right, in bands of 16 pixels.
    let mut ramp = gray_image(128, 64, |x, _| (x * 255 / 127) as u8);
    halftone(8.0).apply_on_cpu(&mut ramp);
    let shares: Vec<f32> = (0..8).map(|band| ink_share(&ramp, band * 16, 0, 16, 64)).collect();
    assert!(shares.windows(2).all(|pair| pair[0] > pair[1]), "{shares:?}");
    assert!(shares[0] > 0.8 && shares[7] < 0.1, "{shares:?}");
  }

  #[test]
  fn half_gray_is_about_half_ink_made_of_dots() {
    // The dot grows with the share of the cell that is dark, so half gray is about half ink.
    let mut gray = gray_image(96, 96, |_, _| 128);
    halftone(12.0).apply_on_cpu(&mut gray);
    let share = ink_share(&gray, 0, 0, 96, 96);
    assert!((share - 0.5).abs() < 0.12, "{share}");
    // It is made of black and white pixels, and only a few in between, along the edges of the dots.
    let rgba = gray.rgba();
    let between = rgba.chunks_exact(4).filter(|pixel| pixel[0] > 25 && pixel[0] < 230).count();
    assert!(between < rgba.len() / 4 / 4, "{between}");
  }

  #[test]
  fn the_ink_and_the_paper_are_used_and_the_alpha_is_kept() {
    let clear: Vec<u8> = (0..24 * 24).flat_map(|_| [255, 255, 255, 90]).collect();
    let mut image = Image::new_from_pixels(24, 24, clear, Channels::RGBA);
    halftone(8.0).with_ink(Color::from_rgb(255, 0, 0)).with_paper(Color::from_rgb(0, 0, 255)).apply_on_cpu(&mut image);
    // A white picture is all paper, here blue, and the alpha is the picture's.
    assert!(image.rgba().chunks_exact(4).all(|pixel| pixel[0] == 0 && pixel[2] == 255 && pixel[3] == 90));
    let dark: Vec<u8> = (0..24 * 24).flat_map(|_| [0, 0, 0, 255]).collect();
    let mut black = Image::new_from_pixels(24, 24, dark, Channels::RGBA);
    halftone(8.0).with_ink(Color::from_rgb(255, 0, 0)).with_paper(Color::from_rgb(0, 0, 255)).apply_on_cpu(&mut black);
    assert!(black.rgba().chunks_exact(4).all(|pixel| pixel[0] == 255 && pixel[2] == 0));
  }

  #[test]
  fn at_no_angle_the_dots_are_in_upright_rows() {
    // The dots are centered on multiples of the spacing, so two rows of dots in a light picture have their dots in the same columns.
    let mut upright = gray_image(64, 64, |_, _| 190);
    halftone(8.0).with_angle(0.0).apply_on_cpu(&mut upright);
    let rgba = upright.rgba();
    let dark_columns = |y: u32| -> Vec<u32> { (0..64).filter(|&x| rgba[((y * 64 + x) * 4) as usize] < 128).collect() };
    assert!(!dark_columns(8).is_empty());
    assert_eq!(dark_columns(8), dark_columns(16));
  }

  #[test]
  fn the_gpu_makes_the_same_screen_as_the_cpu() -> anyhow::Result<()> {
    use gpu::{GpuContext, LiveRenderer};

    let (width, height) = (53u32, 41u32);
    let pixels: Vec<u8> = (0..width * height)
      .flat_map(|index| {
        let (x, y) = (index % width, index / width);
        let gray = ((x * 255 / (width - 1)) as u8).wrapping_add((y * 5) as u8);
        [gray, gray / 2, 255 - gray, 255]
      })
      .collect();
    let effect = halftone(9.0).with_angle(30.0).with_ink(Color::from_rgb(200, 30, 60)).with_paper(Color::from_rgb(255, 240, 224));
    let mut renderer = LiveRenderer::new(GpuContext::new_default_blocking()?);
    let gpu = renderer.process(&[effect.passes(width, height)], width, height, &pixels)?;
    let mut cpu = Image::new_from_pixels(width, height, pixels, Channels::RGBA);
    effect.apply_on_cpu(&mut cpu);
    let cpu = cpu.to_rgba_vec();
    // The float sums are not identical, so a pixel on the edge of a dot may differ by a few levels, and now and then a pixel on a cell's
    // border may take the next cell's dot.
    let far = gpu.chunks_exact(4).zip(cpu.chunks_exact(4)).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8)).count();
    assert!(far <= (width * height) as usize / 50, "{far} of {} pixels differ by more than a few levels", width * height);
    Ok(())
  }
}
