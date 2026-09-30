use crate::apply_adjustment;
use abra_core::image::gpu::{CpuProcessor, GpuAux, GpuPass, GpuProcessor};
use abra_core::{Gradient, Image, ImageRef};
use options::{Apply, Options};

/// Number of colors sampled from the gradient into the lookup table the shader reads.
const LUT_SIZE: u32 = 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
  Angle(f32),
  Between((f32, f32), (f32, f32)),
}

/// A linear gradient drawn over the image on the GPU.
///
/// The gradient stays parametric: changing the angle, endpoints or opacity only changes a few uniform bytes, so it can
/// be re-rendered every frame while a slider is dragged. Colors are composited source-over.
#[derive(Clone)]
pub struct LinearGradientEffect {
  lut: Vec<u8>,
  direction: Direction,
  opacity: f32,
  options: Options,
}

impl LinearGradientEffect {
  /// A gradient whose line crosses the image center at `p_degrees`, sized so the first stop touches one corner and
  /// the last stop the opposite one. Follows the CSS convention: 0 points up and angles increase clockwise, so 90
  /// runs left to right.
  pub fn angle(p_gradient: &Gradient, p_degrees: f32) -> LinearGradientEffect {
    LinearGradientEffect {
      lut: build_lut(p_gradient),
      direction: Direction::Angle(p_degrees),
      opacity: 1.0,
      options: None,
    }
  }

  /// A gradient from `p_start` to `p_end`, in image pixels.
  pub fn between(p_gradient: &Gradient, p_start: (f32, f32), p_end: (f32, f32)) -> LinearGradientEffect {
    LinearGradientEffect {
      lut: build_lut(p_gradient),
      direction: Direction::Between(p_start, p_end),
      opacity: 1.0,
      options: None,
    }
  }

  /// Sets the overall opacity, from 0 to 1.
  pub fn with_opacity(mut self, p_opacity: f32) -> LinearGradientEffect {
    self.opacity = p_opacity;
    self
  }

  /// Turns the gradient to `p_degrees`. See [`angle`](Self::angle).
  pub fn set_angle(&mut self, p_degrees: f32) {
    self.direction = Direction::Angle(p_degrees);
  }

  /// Moves the gradient line to run from `p_start` to `p_end`, in image pixels.
  pub fn set_between(&mut self, p_start: (f32, f32), p_end: (f32, f32)) {
    self.direction = Direction::Between(p_start, p_end);
  }

  /// Sets the overall opacity, from 0 to 1.
  pub fn set_opacity(&mut self, p_opacity: f32) {
    self.opacity = p_opacity;
  }

  /// Replaces the colors.
  pub fn set_gradient(&mut self, p_gradient: &Gradient) {
    self.lut = build_lut(p_gradient);
  }

  /// The start and end of the gradient line for an image of the given size.
  fn line(&self, p_width: u32, p_height: u32) -> ((f32, f32), (f32, f32)) {
    match self.direction {
      Direction::Between(start, end) => (start, end),
      Direction::Angle(degrees) => {
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        let (width, height) = (p_width as f32, p_height as f32);
        let half = (width * sin.abs() + height * cos.abs()) / 2.0;
        let (center_x, center_y) = (width / 2.0, height / 2.0);
        // Image y grows downward, so "up" is negative y.
        let (dx, dy) = (sin, -cos);
        ((center_x - dx * half, center_y - dy * half), (center_x + dx * half, center_y + dy * half))
      }
    }
  }
}

impl GpuProcessor for LinearGradientEffect {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let (start, end) = self.line(p_width, p_height);
    let params = [start.0, start.1, end.0, end.1, self.opacity.clamp(0.0, 1.0), LUT_SIZE as f32, 0.0, 0.0];
    let uniforms: Vec<u8> = params.iter().flat_map(|value| value.to_le_bytes()).collect();
    vec![GpuPass::new(include_str!("./linear_gradient.wgsl"), uniforms).with_aux(GpuAux {
      width: LUT_SIZE,
      height: 1,
      rgba: self.lut.as_slice().into(),
    })]
  }
}

impl CpuProcessor for LinearGradientEffect {
  fn process(&self, p_image: &mut Image) {
    self.apply_to_image(p_image);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl LinearGradientEffect {
  /// The same math as the shader: straight-alpha source-over of the sampled gradient. This is the whole CPU
  /// implementation; [`CpuProcessor::process`] adds the effect's area and mask around it.
  fn draw(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return;
    }
    let (start, end) = self.line(width, height);
    let axis = (end.0 - start.0, end.1 - start.1);
    let axis_len_sq = axis.0 * axis.0 + axis.1 * axis.1;
    let opacity = self.opacity.clamp(0.0, 1.0);
    let mut pixels = p_image.to_rgba_vec();
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
      let (x, y) = ((index as u32 % width) as f32 + 0.5, (index as u32 / width) as f32 + 0.5);
      let t = if axis_len_sq > 0.0 {
        (((x - start.0) * axis.0 + (y - start.1) * axis.1) / axis_len_sq).clamp(0.0, 1.0)
      } else {
        0.0
      };
      let lut_index = (t * (LUT_SIZE - 1) as f32).round() as usize * 4;
      let gradient = &self.lut[lut_index..lut_index + 4];
      let src_a = gradient[3] as f32 / 255.0 * opacity;
      let base_a = pixel[3] as f32 / 255.0;
      let out_a = src_a + base_a * (1.0 - src_a);
      for channel in 0..3 {
        let value = if out_a > 0.0 {
          (gradient[channel] as f32 * src_a + pixel[channel] as f32 * base_a * (1.0 - src_a)) / out_a
        } else {
          0.0
        };
        pixel[channel] = value.round().clamp(0.0, 255.0) as u8;
      }
      pixel[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    p_image.set_rgba(pixels);
  }
}

impl Apply for LinearGradientEffect {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn apply_to_image<'a>(&self, p_image: impl Into<ImageRef<'a>>) {
    let mut image_ref: ImageRef = p_image.into();
    let image = &mut image_ref as &mut Image;
    apply_adjustment!(gpu = self; draw_gradient, image, self.options.as_ref(), 0, self);
  }
}

fn draw_gradient(p_image: &mut Image, p_effect: &LinearGradientEffect) {
  p_effect.draw(p_image);
}

/// Samples the gradient into `LUT_SIZE` RGBA texels so the shader needs only a lookup per pixel.
fn build_lut(p_gradient: &Gradient) -> Vec<u8> {
  let mut lut = Vec::with_capacity(LUT_SIZE as usize * 4);
  for index in 0..LUT_SIZE {
    let (r, g, b, a) = p_gradient.color_at(index as f32 / (LUT_SIZE - 1) as f32).rgba();
    lut.extend_from_slice(&[r, g, b, a]);
  }
  lut
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;
  use gpu::{GpuContext, LiveRenderer};

  const WIDTH: u32 = 37;
  const HEIGHT: u32 = 21;

  fn base_pixels() -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..HEIGHT {
      for x in 0..WIDTH {
        pixels.extend_from_slice(&[(x * 6) as u8, (y * 12) as u8, 90, 255]);
      }
    }
    pixels
  }

  fn renderer() -> anyhow::Result<LiveRenderer> {
    Ok(LiveRenderer::new(GpuContext::new_default_blocking()?))
  }

  /// The same math as the shader, on the CPU.
  fn expected(p_effect: &LinearGradientEffect, p_gradient: &Gradient, p_base: &[u8]) -> Vec<u8> {
    let (start, end) = p_effect.line(WIDTH, HEIGHT);
    let axis = (end.0 - start.0, end.1 - start.1);
    let axis_len_sq = axis.0 * axis.0 + axis.1 * axis.1;
    let mut out = Vec::new();
    for (index, base) in p_base.chunks_exact(4).enumerate() {
      let (x, y) = ((index as u32 % WIDTH) as f32 + 0.5, (index as u32 / WIDTH) as f32 + 0.5);
      let t = (((x - start.0) * axis.0 + (y - start.1) * axis.1) / axis_len_sq).clamp(0.0, 1.0);
      let sampled = (t * (LUT_SIZE - 1) as f32).round() / (LUT_SIZE - 1) as f32;
      let (r, g, b, a) = p_gradient.color_at(sampled).rgba();
      let alpha = a as f32 / 255.0 * p_effect.opacity;
      let mix = |gradient: u8, base: u8| (gradient as f32 * alpha + base as f32 * (1.0 - alpha)).round() as u8;
      out.extend_from_slice(&[mix(r, base[0]), mix(g, base[1]), mix(b, base[2]), 255]);
    }
    out
  }

  fn assert_close(p_actual: &[u8], p_expected: &[u8]) {
    assert_eq!(p_actual.len(), p_expected.len());
    for (index, (a, e)) in p_actual.iter().zip(p_expected).enumerate() {
      assert!(a.abs_diff(*e) <= 2, "byte {index}: gpu {a} vs cpu {e}");
    }
  }

  #[test]
  fn matches_the_cpu_math_at_several_angles_and_opacities() -> anyhow::Result<()> {
    let mut renderer = renderer()?;
    let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
    let base = base_pixels();
    for (angle, opacity) in [(0.0, 1.0), (90.0, 1.0), (33.0, 1.0), (215.0, 0.5), (90.0, 0.0)] {
      let effect = LinearGradientEffect::angle(&gradient, angle).with_opacity(opacity);
      let out = renderer.process(&[&effect], WIDTH, HEIGHT, &base)?;
      assert_close(&out, &expected(&effect, &gradient, &base));
    }
    Ok(())
  }

  #[test]
  fn ninety_degrees_runs_left_to_right() -> anyhow::Result<()> {
    let mut renderer = renderer()?;
    let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
    let effect = LinearGradientEffect::angle(&gradient, 90.0);
    let out = renderer.process(&[&effect], WIDTH, HEIGHT, &base_pixels())?;
    let row = 10 * WIDTH as usize * 4;
    assert!(out[row] > 240 && out[row + 2] < 15, "left edge should be the first stop");
    let last = row + (WIDTH as usize - 1) * 4;
    assert!(out[last] < 15 && out[last + 2] > 240, "right edge should be the last stop");
    Ok(())
  }

  #[test]
  fn dragging_the_angle_re_renders_without_rebuilding_anything() -> anyhow::Result<()> {
    let mut renderer = renderer()?;
    let gradient = Gradient::from_to(Color::black(), Color::white());
    let mut effect = LinearGradientEffect::angle(&gradient, 0.0);
    renderer.set_source(WIDTH, HEIGHT, &base_pixels())?;

    let mut frames = Vec::new();
    for angle in [0.0, 30.0, 60.0, 90.0] {
      effect.set_angle(angle);
      renderer.render(&[&effect])?;
      frames.push(renderer.read_pixels()?);
    }

    assert_eq!(renderer.cached_pipelines(), 1);
    for pair in frames.windows(2) {
      assert_ne!(pair[0], pair[1]);
    }
    Ok(())
  }

  #[test]
  fn a_gradient_with_equal_endpoints_does_not_produce_nan_garbage() -> anyhow::Result<()> {
    let mut renderer = renderer()?;
    let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
    let effect = LinearGradientEffect::between(&gradient, (5.0, 5.0), (5.0, 5.0));
    let out = renderer.process(&[&effect], WIDTH, HEIGHT, &base_pixels())?;
    // With no line, every pixel takes the first stop.
    for pixel in out.chunks_exact(4) {
      assert_close(pixel, &[255, 0, 0, 255]);
    }
    Ok(())
  }
}
