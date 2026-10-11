use abra_core::image::gpu::{GpuAux, GpuPass, GpuProcessor};
use abra_core::{Gradient, Image};
use options::{Effect, Options};

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
  lut: std::sync::Arc<[u8]>,
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
      lut: build_lut(p_gradient).into(),
      direction: Direction::Angle(p_degrees),
      opacity: 1.0,
      options: None,
    }
  }

  /// A gradient from `p_start` to `p_end`, in image pixels.
  pub fn between(p_gradient: &Gradient, p_start: (f32, f32), p_end: (f32, f32)) -> LinearGradientEffect {
    LinearGradientEffect {
      lut: build_lut(p_gradient).into(),
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
    self.lut = build_lut(p_gradient).into();
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

/// Where each pixel falls along the gradient, and how opaque it is, in whole numbers. The CPU and the shader both work
/// from these, with the same steps, so they give exactly the same pixels.
///
/// The position along the gradient is a straight line in the pixel's coordinates: `a * x + b * y + c`, in units of
/// `2^-shift` texels of the color table. `shift` is as large as fits without overflowing, so long and short gradients
/// both keep their precision.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Plan {
  a: i32,
  b: i32,
  c: i32,
  shift: u32,
  /// The overall opacity, 0 to 255.
  opacity: u32,
}

impl Plan {
  /// The index into the color table for the pixel at (`p_x`, `p_y`), from 0 to `LUT_SIZE - 1`.
  fn table_index(&self, p_x: i32, p_y: i32) -> usize {
    let half = (1i32 << self.shift) >> 1;
    let position = self.a * p_x + self.b * p_y + self.c;
    ((position + half) >> self.shift).clamp(0, LUT_SIZE as i32 - 1) as usize
  }
}

/// Puts one gradient color over one pixel, source-over with straight (not premultiplied) alpha, in whole numbers.
/// Both are RGBA bytes, and `p_opacity` is 0 to 255.
fn composite(p_base: [u8; 4], p_gradient: [u8; 4], p_opacity: u32) -> [u8; 4] {
  let source_alpha = (p_gradient[3] as u32 * p_opacity + 127) / 255;
  let base_alpha = p_base[3] as u32;
  let out_alpha = source_alpha + (base_alpha * (255 - source_alpha) + 127) / 255;
  if out_alpha == 0 {
    return [0, 0, 0, 0];
  }
  let divisor = out_alpha * 255;
  let channel = |gradient: u8, base: u8| {
    let sum = gradient as u32 * source_alpha * 255 + base as u32 * base_alpha * (255 - source_alpha);
    ((sum + divisor / 2) / divisor).min(255) as u8
  };
  [
    channel(p_gradient[0], p_base[0]),
    channel(p_gradient[1], p_base[1]),
    channel(p_gradient[2], p_base[2]),
    out_alpha as u8,
  ]
}

impl LinearGradientEffect {
  /// The whole-number plan for an image of the given size.
  fn plan(&self, p_width: u32, p_height: u32) -> Plan {
    let opacity = (self.opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    let (start, end) = self.line(p_width, p_height);
    let (start_x, start_y) = (start.0 as f64, start.1 as f64);
    let axis = (end.0 as f64 - start_x, end.1 as f64 - start_y);
    let length_squared = axis.0 * axis.0 + axis.1 * axis.1;
    if length_squared.is_nan() || length_squared <= 0.0 {
      // No line: every pixel takes the first color.
      return Plan {
        a: 0,
        b: 0,
        c: 0,
        shift: 0,
        opacity,
      };
    }
    // The index is `(x + 0.5 - start_x) * axis.0 + (y + 0.5 - start_y) * axis.1`, scaled to the table, as `a x + b y + c`.
    let scale = (LUT_SIZE - 1) as f64 / length_squared;
    let (mut a, mut b) = (axis.0 * scale, axis.1 * scale);
    let mut c = ((0.5 - start_x) * axis.0 + (0.5 - start_y) * axis.1) * scale;
    // Keep every value below 2^29 before shifting, so nothing overflows; a gradient too short to fit only loses
    // resolution it never had.
    let largest = a.abs() * p_width as f64 + b.abs() * p_height as f64 + c.abs();
    let limit = (1u64 << 29) as f64;
    if largest > limit {
      let factor = limit / largest;
      (a, b, c) = (a * factor, b * factor, c * factor);
    }
    let largest = largest.min(limit).max(1.0);
    let mut shift = 0u32;
    while shift < 20 && largest * (1u64 << (shift + 1)) as f64 <= (1u64 << 30) as f64 {
      shift += 1;
    }
    let fixed = |value: f64| (value * (1u64 << shift) as f64).round() as i32;
    Plan {
      a: fixed(a),
      b: fixed(b),
      c: fixed(c),
      shift,
      opacity,
    }
  }
}

impl GpuProcessor for LinearGradientEffect {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let plan = self.plan(p_width, p_height);
    let mut uniforms: Vec<u8> = Vec::with_capacity(32);
    for value in [
      plan.a as u32,
      plan.b as u32,
      plan.c as u32,
      plan.shift,
      plan.opacity,
      0,
      0,
      0,
    ] {
      uniforms.extend_from_slice(&value.to_le_bytes());
    }
    vec![
      GpuPass::new(include_str!("./linear_gradient.wgsl"), uniforms).with_aux(GpuAux {
        width: LUT_SIZE,
        height: 1,
        rgba: self.lut.clone(),
      }),
    ]
  }
}

impl LinearGradientEffect {
  /// The whole CPU implementation, in the same whole-number steps as the shader: source-over of the sampled gradient.
  /// [`Apply`] adds the effect's area and mask around it.
  fn draw(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return;
    }
    let plan = self.plan(width, height);
    let mut pixels = p_image.to_rgba_vec();
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
      let (x, y) = ((index as u32 % width) as i32, (index as u32 / width) as i32);
      let table = plan.table_index(x, y) * 4;
      let gradient = [
        self.lut[table],
        self.lut[table + 1],
        self.lut[table + 2],
        self.lut[table + 3],
      ];
      pixel.copy_from_slice(&composite([pixel[0], pixel[1], pixel[2], pixel[3]], gradient, plan.opacity));
    }
    p_image.set_rgba(pixels);
  }
}

impl Effect for LinearGradientEffect {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn padding(&self) -> i32 {
    0
  }

  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    draw_gradient(p_image, self);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
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
      let out = renderer.process(&[effect.passes(WIDTH, HEIGHT)], WIDTH, HEIGHT, &base)?;
      assert_close(&out, &expected(&effect, &gradient, &base));
    }
    Ok(())
  }

  #[test]
  fn ninety_degrees_runs_left_to_right() -> anyhow::Result<()> {
    let mut renderer = renderer()?;
    let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
    let effect = LinearGradientEffect::angle(&gradient, 90.0);
    let out = renderer.process(&[effect.passes(WIDTH, HEIGHT)], WIDTH, HEIGHT, &base_pixels())?;
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
      renderer.render(&[effect.passes(WIDTH, HEIGHT)])?;
      frames.push(renderer.read_pixels()?);
    }

    assert_eq!(renderer.cached_pipelines(), 1);
    for pair in frames.windows(2) {
      assert_ne!(pair[0], pair[1]);
    }
    Ok(())
  }

  /// The table index for the far corner and the first pixel, without overflowing, whatever the size and line.
  #[test]
  fn the_position_math_never_overflows_and_stays_in_the_table() {
    let gradient = Gradient::from_to(Color::black(), Color::white());
    let last = LUT_SIZE as usize - 1;
    for (width, height) in [(1, 1), (37, 21), (1920, 1080), (8192, 8192), (16384, 16384)] {
      let lines = [
        ((0.0, 0.0), (width as f32, height as f32)),
        ((width as f32, height as f32), (0.0, 0.0)),
        ((5.0, 5.0), (5.5, 5.0)),
        ((5.0, 5.0), (5.0, 5.001)),
        ((-1.0e6, -1.0e6), (1.0e6, 1.0e6)),
        ((0.0, 0.0), (1.0e-6, 0.0)),
      ];
      for (start, end) in lines {
        let plan = LinearGradientEffect::between(&gradient, start, end).plan(width, height);
        for (x, y) in [
          (0, 0),
          (width as i32 - 1, 0),
          (0, height as i32 - 1),
          (width as i32 - 1, height as i32 - 1),
        ] {
          assert!(plan.table_index(x, y) <= last, "{width}x{height} from {start:?} to {end:?} at ({x}, {y})");
        }
      }
    }
  }

  #[test]
  fn a_long_gradient_still_runs_from_the_first_color_to_the_last() {
    let gradient = Gradient::from_to(Color::black(), Color::white());
    let plan = LinearGradientEffect::between(&gradient, (0.0, 0.0), (8000.0, 0.0)).plan(8192, 1);
    assert_eq!(plan.table_index(0, 0), 0);
    assert_eq!(plan.table_index(8000, 0), LUT_SIZE as usize - 1);
    let middle = plan.table_index(4000, 0) as i32;
    assert!((middle - (LUT_SIZE as i32 - 1) / 2).abs() <= 1, "halfway along is halfway through the table: {middle}");
  }

  #[test]
  fn a_gradient_with_equal_endpoints_does_not_produce_nan_garbage() -> anyhow::Result<()> {
    let mut renderer = renderer()?;
    let gradient = Gradient::from_to(Color::from_rgba(255, 0, 0, 255), Color::from_rgba(0, 0, 255, 255));
    let effect = LinearGradientEffect::between(&gradient, (5.0, 5.0), (5.0, 5.0));
    let out = renderer.process(&[effect.passes(WIDTH, HEIGHT)], WIDTH, HEIGHT, &base_pixels())?;
    // With no line, every pixel takes the first stop.
    for pixel in out.chunks_exact(4) {
      assert_close(pixel, &[255, 0, 0, 255]);
    }
    Ok(())
  }
}
