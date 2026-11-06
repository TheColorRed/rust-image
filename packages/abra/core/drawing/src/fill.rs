use std::borrow::Cow;
use std::sync::Arc;

use abra_core::image::gpu::{GpuAux, GpuPass, Hardware, ready_gpu_provider};
use abra_core::image::recipe::{Generator, blended, generated};
use abra_core::{Area, BlendMode, ColorSpace, Fill, Image, ImageRepeat, Path, PointF};
use rayon::prelude::*;

use crate::core::shader::gradient_path;
use crate::shaders::fill_feather_shader::FillFeatherShader;
use crate::shaders::image_shader::repeated_sample;
use crate::{PolygonCoverage, Rasterizer, SampleGrid, SourceOverCompositor, shader_from_fill_with_path};

/// Most points a shape is drawn from on the GPU. A shape with more is drawn on the CPU, so the shader's uniform buffer has a fixed
/// size and costs a bounded amount per pixel.
pub const MAX_FILL_POINTS: usize = 1024;
/// Most color stops a gradient has when it is drawn on the GPU.
pub const MAX_GRADIENT_STOPS: usize = 16;
/// Most points the line of a gradient has when it is drawn on the GPU.
pub const MAX_GRADIENT_POINTS: usize = 64;
/// The longest side of an image or mask that is drawn on the GPU.
const MAX_TEXTURE_SIDE: u32 = 8192;

/// What it costs to draw a fill, in milliseconds, measured with `timing_of_gpu_and_cpu_fills` on a desktop. They decide, in
/// [`FillArea::apply`] and nowhere else, whether the GPU or the CPU draws a fill. The CPU draws only the pixels the fill's bounds cover, but
/// each costs something. The GPU draws the whole target however small the fill is, and the result is read back as a whole, so it costs about
/// the same for any fill and more for a bigger target. A fill is drawn where it is cheaper. Adjust them when measuring on another device.
mod cost {
  /// The CPU's fixed cost for any fill.
  pub(super) const CPU_FIXED: f32 = 0.4;
  /// The CPU's cost for each pixel a solid fill's bounds cover.
  pub(super) const CPU_PER_SOLID_PIXEL: f32 = 0.0001;
  /// The CPU's cost for each pixel a gradient or image fill's bounds cover, which works out a color for each.
  pub(super) const CPU_PER_SHADED_PIXEL: f32 = 0.0002;
  /// The GPU's fixed cost for a run.
  pub(super) const GPU_FIXED: f32 = 3.0;
  /// The GPU's cost for each pixel of the target.
  pub(super) const GPU_PER_TARGET_PIXEL: f32 = 0.00001;
}

/// What the cost of drawing a fill on the GPU depends on, for a target that can take one there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuState {
  /// The width and height of the target in pixels.
  pub size: (u32, u32),
  /// Whether GPU work is already waiting on the target. Another pass then costs little, where drawing on the CPU would make the target
  /// run it now.
  pub pending: bool,
}

/// A fill that has been described but not yet drawn. Create one with [`fill`], then either draw it into an existing
/// image with [`FillArea::apply`] or get it as a new image with [`FillArea::to_image`] or `into()`.
#[derive(Clone)]
pub struct FillArea<'a> {
  area: Area,
  fill: Fill<'a>,
  position: Option<PointF>,
  mask: Option<Image>,
}

/// A target that can receive the rasterized result of a fill.
pub trait FillTarget {
  /// Draws a rasterized fill at its requested position.
  fn draw_fill(&mut self, p_image: &Image, p_position: (i32, i32));

  /// What the GPU would cost for this target, or `None` (the default) when it cannot take a fill there.
  fn gpu_state(&self) -> Option<GpuState> {
    None
  }

  /// Draws `p_fill` with the GPU and returns `true`, or returns `false` (the default) to have it rasterized on the CPU and
  /// drawn with [`FillTarget::draw_fill`]. [`FillArea::apply`] asks only when the GPU is the cheaper way (see [`FillTarget::gpu_state`]).
  fn draw_fill_on_gpu(&mut self, _p_fill: &FillArea<'_>) -> bool {
    false
  }
}

impl FillTarget for Image {
  /// Composites the fill over the image, so the clear pixels around a shape leave what is under them as it was.
  fn draw_fill(&mut self, p_image: &Image, p_position: (i32, i32)) {
    abra_core::blend::blend(p_image).with_offset(PointF::new(p_position.0 as f32, p_position.1 as f32)).apply(self);
  }

  fn gpu_state(&self) -> Option<GpuState> {
    Some(GpuState { size: self.dimensions::<u32>(), pending: self.deferred().is_some() })
  }

  fn draw_fill_on_gpu(&mut self, p_fill: &FillArea<'_>) -> bool {
    let (width, height) = self.dimensions::<u32>();
    let Some(canvas) = p_fill.gpu_canvas(width, height) else { return false };
    *self = blended(self, &canvas, BlendMode::Normal, 1.0);
    true
  }
}

impl<'a> FillArea<'a> {
  /// Sets where [`FillArea::apply`] draws the top-left corner of the filled area. Defaults to the top-left corner
  /// of the area's bounds, so the area lands where it was drawn.
  pub fn with_position(mut self, p_position: impl Into<PointF>) -> Self {
    self.position = Some(p_position.into());
    self
  }

  /// Multiplies fill opacity by a grayscale mask. The mask's red channel is used and its dimensions must match the
  /// rasterized area's bounds. A `Mask` can be passed directly through its conversion into `Image`.
  pub fn with_mask(mut self, p_mask: impl Into<Image>) -> Self {
    self.mask = Some(p_mask.into());
    self
  }

  /// Rasterizes the area into a new image the size of its bounds.
  pub fn to_image(&self) -> Image {
    let area = &self.area;
    let (min_x, min_y, max_x, max_y) = area.bounds().edges::<f32>();
    let width = (max_x - min_x).ceil();
    let height = (max_y - min_y).ceil();

    if width <= 0.0 || height <= 0.0 {
      return Image::new(1, 1);
    }

    let mut image = Image::new(width as u32, height as u32);

    // Flatten the path and translate to image-local coordinates
    let tolerance = 0.5;
    let flattened: Vec<PointF> =
      area.path.flatten(tolerance).iter().map(|p| PointF::new(p.x - min_x, p.y - min_y)).collect();

    // Build coverage mask
    let coverage = PolygonCoverage::new(flattened.clone());

    // Build shader from fill. If the gradient has no explicit direction, use the
    // area bounding box to create a horizontal gradient path so the gradient
    // is visible across the area.
    let fallback_path = Some(Path::from(Area::rect((0.0, 0.0), (width, height))));
    let mut shader = shader_from_fill_with_path(self.fill.clone(), fallback_path);
    // Apply area feathering by wrapping the shader when area has feather set
    if area.feather() > 0 {
      // max_distance is in pixels
      shader = Box::new(FillFeatherShader::new_from_flattened(shader, flattened.clone(), area.feather() as f32));
    }

    // Use source-over compositing
    let compositor = SourceOverCompositor;

    // Use anti-aliasing level from image
    let sample_grid = SampleGrid::from_aa_level(image.anti_aliasing_level);

    // Rasterize
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);
    rasterizer.rasterize(&mut image);

    if let Some(mask) = &self.mask {
      assert_eq!(
        mask.dimensions::<u32>(),
        image.dimensions::<u32>(),
        "Fill mask dimensions must match the rasterized area bounds"
      );
      let mask_pixels = mask.rgba();
      let pixels = image.colors().as_slice_mut().expect("Image colors must be contiguous");
      pixels.par_chunks_exact_mut(4).zip(mask_pixels.par_chunks_exact(4)).for_each(|(pixel, mask_pixel)| {
        pixel[3] = ((pixel[3] as u16 * mask_pixel[0] as u16 + 127) / 255) as u8;
      });
    }

    image
  }

  /// Draws the filled area into the specified target, at the position set by [`FillArea::with_position`] or else where the area is.
  ///
  /// This is the one place that chooses between the GPU and the CPU, so a caller only says what to draw. The GPU does it when it is the
  /// cheaper way (a fill that covers much of a target, or a target that is already waiting on the GPU: see `cost`), the target can take it
  /// (an image, with a GPU ready) and the fill fits the shader: any solid color, gradient or image fill, with or without a mask or
  /// feathering, of a shape of up to [`MAX_FILL_POINTS`] points. The image is then not changed until its pixels are read, so a run of fills,
  /// blends and filters is one GPU run. Anything else, or no GPU, is drawn now on the CPU. Both give the same pixels, to within a level or
  /// two of rounding.
  pub fn apply(&self, p_target: &mut impl FillTarget) {
    if let Some(state) = p_target.gpu_state()
      && self.cheaper_on_gpu(state)
      && p_target.draw_fill_on_gpu(self)
    {
      return;
    }
    self.apply_on_cpu(p_target);
  }

  /// Whether the GPU draws this fill onto a target in `p_state` for less than the CPU does, by the numbers in `cost`.
  fn cheaper_on_gpu(&self, p_state: GpuState) -> bool {
    if p_state.pending {
      return true;
    }
    let (width, height) = p_state.size;
    let target_pixels = width as f32 * height as f32;
    let (min_x, min_y, max_x, max_y) = self.area.bounds().edges::<f32>();
    // The part of the bounds that is on the target is all the CPU draws.
    let covered = ((max_x.min(width as f32) - min_x.max(0.0)).max(0.0)) * ((max_y.min(height as f32) - min_y.max(0.0)).max(0.0));
    let per_pixel = match self.fill {
      Fill::Solid(_) => cost::CPU_PER_SOLID_PIXEL,
      Fill::Gradient(_) | Fill::Image(..) => cost::CPU_PER_SHADED_PIXEL,
    };
    let on_the_cpu = |p_pixels: f32| cost::CPU_FIXED + p_pixels * per_pixel;
    let on_the_gpu = cost::GPU_FIXED + target_pixels * cost::GPU_PER_TARGET_PIXEL;
    if on_the_cpu(covered) <= on_the_gpu {
      return false;
    }
    // The bounds of a thin or sparse shape, such as a line across the target, are mostly empty and overstate the CPU's work, so the shape
    // itself is measured before the GPU is chosen.
    on_the_cpu(covered.min(self.shape_pixels())) > on_the_gpu
  }

  /// About how many pixels the CPU has work on for this shape: the area its outline encloses, and the pixels along the outline.
  fn shape_pixels(&self) -> f32 {
    let points = self.area.path.flatten(0.5);
    let mut twice_area = 0.0f32;
    let mut perimeter = 0.0f32;
    for (index, point) in points.iter().enumerate() {
      let next = points[(index + 1) % points.len()];
      twice_area += point.x * next.y - next.x * point.y;
      perimeter += point.distance_to(next);
    }
    twice_area.abs() / 2.0 + perimeter
  }

  /// [`FillArea::apply`] always on the CPU, whatever the target and the hardware.
  pub fn apply_on_cpu(&self, p_target: &mut impl FillTarget) {
    let filled = self.to_image();
    p_target.draw_fill(&filled, self.drawn_at());
  }

  /// Where the top-left corner of the area is drawn, in whole pixels.
  fn drawn_at(&self) -> (i32, i32) {
    let position = self.position.unwrap_or_else(|| {
      let (min_x, min_y, _, _) = self.area.bounds().edges::<f32>();
      PointF::new(min_x, min_y)
    });
    position.into()
  }

  /// This fill with nothing borrowed, so a recipe can keep it.
  fn owned(&self) -> FillArea<'static> {
    let fill = match &self.fill {
      Fill::Solid(color) => Fill::Solid(*color),
      Fill::Gradient(gradient) => Fill::Gradient(Cow::Owned(gradient.clone().into_owned())),
      Fill::Image(image, repeat) => Fill::Image(image.clone(), *repeat),
    };
    FillArea { area: self.area.clone(), fill, position: self.position, mask: self.mask.clone() }
  }

  /// This fill on a transparent canvas of `p_width` x `p_height` pixels that the GPU makes when its pixels are first read, or
  /// `None` when it cannot: no GPU is ready, or the fill does not fit the shader (see [`FillArea::gpu_passes`]).
  fn gpu_canvas(&self, p_width: u32, p_height: u32) -> Option<Image> {
    ready_gpu_provider(Hardware::Auto)?;
    let canvas = FillCanvas { fill: self.owned(), passes: self.gpu_passes()? };
    Some(generated(p_width, p_height, Arc::new(canvas)))
  }

  /// The shader run that draws this fill, or `None` when it does not fit: the shape has more than [`MAX_FILL_POINTS`] points,
  /// a gradient has more than [`MAX_GRADIENT_STOPS`] stops or its line more than [`MAX_GRADIENT_POINTS`] points, an image or mask
  /// is larger than a texture can be, or a mask is not the size of the area (which the CPU refuses too).
  fn gpu_passes(&self) -> Option<Vec<GpuPass>> {
    let (min_x, min_y, max_x, max_y) = self.area.bounds().edges::<f32>();
    let (width, height) = ((max_x - min_x).ceil(), (max_y - min_y).ceil());
    if width <= 0.0 || height <= 0.0 {
      return None;
    }
    let mut points: Vec<PointF> =
      self.area.path.flatten(0.5).iter().map(|point| PointF::new(point.x - min_x, point.y - min_y)).collect();
    // The flattened outline of a large shape has more points than the shader takes, and nearly all of them lie on straight runs.
    // Dropping the ones that are less than a tenth of a pixel off the line through their neighbours leaves a shape that the
    // rasterizer cannot tell apart, in far fewer points.
    for epsilon in [0.01, 0.03, 0.1] {
      if points.len() <= MAX_FILL_POINTS {
        break;
      }
      points = simplified(&points, epsilon);
    }
    if points.len() < 3 || points.len() > MAX_FILL_POINTS {
      return None;
    }

    let mut kind = 0u32;
    let mut color = [0.0f32; 4];
    let mut stops: Vec<[f32; 8]> = Vec::new();
    let mut direction: Vec<PointF> = Vec::new();
    let mut direction_segments = 0u32;
    let mut aux: Option<(u32, u32, Vec<u8>)> = None;
    let mut image_repeat = ImageRepeat::NoRepeat;
    match &self.fill {
      Fill::Solid(solid) => {
        let (r, g, b, a) = solid.rgba();
        color = [r as f32, g as f32, b as f32, a as f32];
      }
      Fill::Gradient(gradient) => {
        kind = 1;
        if gradient.stops().len() > MAX_GRADIENT_STOPS {
          return None;
        }
        // The shader mixes the stops in sRGB. A gradient that mixes in another space is drawn on the CPU, which can.
        if gradient.interpolation().space != ColorSpace::Srgb {
          return None;
        }
        for stop in gradient.stops() {
          let (r, g, b, a) = stop.color.rgba();
          stops.push([r as f32, g as f32, b as f32, a as f32, stop.time, 0.0, 0.0, 0.0]);
        }
        let bounds = Path::from(Area::rect((0.0, 0.0), (width, height)));
        direction = gradient_path(gradient, Some(&bounds)).flatten(1.0);
        if direction.len() > MAX_GRADIENT_POINTS {
          return None;
        }
        // A line that comes back to its start is closed, and has a piece joining its last point to its first.
        let closed = direction.len() >= 2 && direction[0].distance_to(direction[direction.len() - 1]) < 0.1;
        direction_segments = match direction.len() {
          0 | 1 => 0,
          len if closed => len as u32,
          len => len as u32 - 1,
        };
      }
      Fill::Image(image, repeat) => {
        kind = 2;
        let (image_width, image_height) = image.dimensions::<u32>();
        aux = Some((image_width, image_height, image.rgba().to_vec()));
        image_repeat = *repeat;
      }
    }

    let mut masked = 0u32;
    if let Some(mask) = &self.mask {
      if mask.dimensions::<u32>() != (width as u32, height as u32) {
        return None;
      }
      let mask_pixels = mask.rgba();
      match &mut aux {
        // The image fill and the mask both read the area's own pixels, so they are one texture: the image, repeated or clear where
        // it does not reach, with its opacity times the mask. The texture is then the size of the area, so it is not repeated again.
        Some((image_width, image_height, pixels)) => {
          let (area_width, area_height) = (width as usize, height as usize);
          let (source_width, source_height) = (*image_width as i32, *image_height as i32);
          let mut combined = vec![0u8; area_width * area_height * 4];
          for y in 0..area_height {
            for x in 0..area_width {
              let Some((sx, sy)) = repeated_sample(x as i32, y as i32, source_width, source_height, image_repeat) else {
                continue;
              };
              let from = (sy as usize * *image_width as usize + sx as usize) * 4;
              let to = (y * area_width + x) * 4;
              combined[to..to + 3].copy_from_slice(&pixels[from..from + 3]);
              combined[to + 3] = ((pixels[from + 3] as u16 * mask_pixels[to] as u16 + 127) / 255) as u8;
            }
          }
          (*image_width, *image_height, *pixels) = (width as u32, height as u32, combined);
          image_repeat = ImageRepeat::NoRepeat;
        }
        None => {
          masked = 1;
          aux = Some((width as u32, height as u32, mask_pixels.to_vec()));
        }
      }
    }
    let (aux_width, aux_height, aux_pixels) = aux.unwrap_or((1, 1, vec![0; 4]));
    if aux_width > MAX_TEXTURE_SIDE || aux_height > MAX_TEXTURE_SIDE {
      return None;
    }

    let (offset_x, offset_y) = self.drawn_at();
    let mut uniforms: Vec<u8> = Vec::with_capacity(80 + MAX_GRADIENT_STOPS * 32 + MAX_GRADIENT_POINTS * 16 + MAX_FILL_POINTS * 16);
    let floats = |values: &[f32]| values.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>();
    uniforms.extend(floats(&color));
    uniforms.extend(floats(&[offset_x as f32, offset_y as f32, width, height]));
    uniforms.extend([points.len() as u32, kind, stops.len() as u32, direction.len() as u32].into_iter().flat_map(u32::to_le_bytes));
    let repeat_bits = u32::from(image_repeat.repeats_x()) | (u32::from(image_repeat.repeats_y()) << 1);
    uniforms.extend([direction_segments, Image::new(1, 1).anti_aliasing_level, masked, repeat_bits].into_iter().flat_map(u32::to_le_bytes));
    uniforms.extend(floats(&[self.area.feather() as f32, 0.0, 0.0, 0.0]));
    for index in 0..MAX_GRADIENT_STOPS {
      uniforms.extend(floats(&stops.get(index).copied().unwrap_or([0.0; 8])));
    }
    for index in 0..MAX_GRADIENT_POINTS {
      let point = direction.get(index).copied().unwrap_or(PointF::new(0.0, 0.0));
      uniforms.extend(floats(&[point.x, point.y, 0.0, 0.0]));
    }
    for index in 0..MAX_FILL_POINTS {
      let point = points.get(index).copied().unwrap_or(PointF::new(0.0, 0.0));
      uniforms.extend(floats(&[point.x, point.y, 0.0, 0.0]));
    }

    let aux = GpuAux { width: aux_width, height: aux_height, rgba: Arc::from(aux_pixels) };
    Some(vec![GpuPass::new(include_str!("./fill.wgsl"), uniforms).with_aux(aux).with_window()])
  }
}

/// `p_points` (a ring: its last point is joined to its first) without the points that lie within `p_epsilon` pixels of the line
/// between the points kept either side of them (Douglas-Peucker).
fn simplified(p_points: &[PointF], p_epsilon: f32) -> Vec<PointF> {
  let last = p_points.len() - 1;
  if last < 3 {
    return p_points.to_vec();
  }
  let distance_to_segment = |point: PointF, from: PointF, to: PointF| {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let length_squared = dx * dx + dy * dy;
    let along =
      if length_squared > 0.0 { (((point.x - from.x) * dx + (point.y - from.y) * dy) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
    ((point.x - from.x - dx * along).powi(2) + (point.y - from.y - dy * along).powi(2)).sqrt()
  };
  let mut kept = vec![false; p_points.len()];
  kept[0] = true;
  kept[last] = true;
  // A ring starts and ends at the same place, which has no line between them, so it is split at the point farthest from its start.
  let farthest = (1..=last)
    .max_by(|a, b| p_points[0].distance_to(p_points[*a]).total_cmp(&p_points[0].distance_to(p_points[*b])))
    .unwrap_or(last);
  kept[farthest] = true;
  let mut pending = vec![(0, farthest), (farthest, last)];
  while let Some((from, to)) = pending.pop() {
    let worst = (from + 1..to)
      .map(|index| (index, distance_to_segment(p_points[index], p_points[from], p_points[to])))
      .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((index, _)) = worst.filter(|(_, distance)| *distance > p_epsilon) {
      kept[index] = true;
      pending.push((from, index));
      pending.push((index, to));
    }
  }
  p_points.iter().zip(kept).filter(|(_, keep)| *keep).map(|(point, _)| *point).collect()
}

/// A fill drawn on a transparent canvas: the CPU rasterizer is the reference, and the shader does the same steps.
struct FillCanvas {
  fill: FillArea<'static>,
  passes: Vec<GpuPass>,
}

impl std::fmt::Debug for FillCanvas {
  fn fmt(&self, p_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    p_formatter.write_str("FillCanvas")
  }
}

impl Generator for FillCanvas {
  fn render(&self, p_width: u32, p_height: u32) -> Image {
    let mut canvas = Image::new(p_width, p_height);
    self.fill.apply_on_cpu(&mut canvas);
    canvas
  }

  fn passes(&self, _p_width: u32, _p_height: u32) -> Option<Vec<GpuPass>> {
    Some(self.passes.clone())
  }
}

impl From<FillArea<'_>> for Image {
  fn from(p_fill: FillArea<'_>) -> Self {
    p_fill.to_image()
  }
}

/// Fills an area with a color, gradient, or image.
/// # Arguments
/// - `p_area`: The area to fill.
/// - `p_fill_source`: The source fill style to use.
pub fn fill<'a>(p_area: impl Into<Area>, p_fill_source: impl Into<Fill<'a>>) -> FillArea<'a> {
  FillArea {
    area: p_area.into(),
    fill: p_fill_source.into(),
    position: None,
    mask: None,
  }
}

#[cfg(test)]
mod gpu_tests {
  use super::*;
  use abra_core::{Channels, Color, Gradient};

  fn photo(p_width: u32, p_height: u32) -> Image {
    Image::new_from_pixels(p_width, p_height, [90u8, 120, 150, 255].repeat((p_width * p_height) as usize), Channels::RGBA)
  }

  fn patterned(p_width: u32, p_height: u32) -> Image {
    let pixels: Vec<u8> = (0..p_width * p_height)
      .flat_map(|i| [(i * 7 % 256) as u8, (i * 13 % 256) as u8, (i * 29 % 256) as u8, 255 - (i % 90) as u8])
      .collect();
    Image::new_from_pixels(p_width, p_height, pixels, Channels::RGBA)
  }

  /// Every fill kind is drawn by the GPU to the CPU rasterizer's pixels. The real provider is wrapped in one that counts, so the
  /// test fails if the work quietly fell back to the CPU (which would match perfectly).
  #[test]
  fn the_gpu_gives_the_cpu_s_pixels_for_every_kind_of_fill() -> anyhow::Result<()> {
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, GpuProvider, get_gpu_provider, register_gpu_provider};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static REAL: OnceLock<GpuProvider> = OnceLock::new();
    static RUNS: AtomicUsize = AtomicUsize::new(0);

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu::GpuContext::new_default_blocking()?;
    gpu::register();
    let real = *REAL.get_or_init(|| get_gpu_provider().expect("the gpu crate registers a provider"));
    register_gpu_provider(GpuProvider {
      process: |effects, width, height, pixels| {
        RUNS.fetch_add(1, Ordering::SeqCst);
        (REAL.get().unwrap().process)(effects, width, height, pixels)
      },
      ..real
    });
    RUNS.store(0, Ordering::SeqCst);

    let (width, height) = (97, 61);
    let circle = || Area::circle((48.5, 30.0), 22.5);
    let rect = || Area::rect((10.0, 8.0), (30.0, 20.0));
    let triangle = || Area::from_points(&[[5.0, 50.0], [50.0, 4.0], [92.0, 55.0]]);
    let red = Color::from_rgba(240, 30, 60, 255);
    let blue = Color::from_rgba(20, 60, 230, 200);
    let gradient = Gradient::new(vec![
      abra_core::ColorStop::new(red, 0.0),
      abra_core::ColorStop::new(Color::from_rgba(250, 230, 20, 255), 0.4),
      abra_core::ColorStop::new(blue, 1.0),
    ]);
    let mask = Image::new_from_pixels(
      30,
      20,
      (0..30 * 20).flat_map(|i| [(i * 5 % 256) as u8, 0, 0, 255]).collect::<Vec<u8>>(),
      Channels::RGBA,
    );
    let fills: Vec<(&str, FillArea<'static>)> = vec![
      ("a solid circle", fill(circle(), red)),
      ("a solid triangle", fill(triangle(), blue)),
      ("a gradient", fill(rect(), Fill::gradient(Cow::Owned(gradient.clone())))),
      ("an angled gradient", fill(triangle(), Fill::gradient(Cow::Owned(gradient.clone().with_direction(Path::line((5.0, 50.0), (92.0, 4.0))))))),
      ("an image", fill(rect(), Fill::image(Arc::new(patterned(12, 9))))),
      ("a mask", fill(rect(), red).with_mask(mask.clone())),
      ("a feathered edge", fill(Area::circle((48.5, 30.0), 22.5).with_feather(7), red)),
      ("a feathered gradient", fill(rect().with_feather(5), Fill::gradient(Cow::Owned(gradient.clone())))),
      ("an image and a mask", fill(rect(), Fill::image(Arc::new(patterned(40, 30)))).with_mask(mask)),
      ("a repeating image", fill(rect(), Fill::image(Arc::new(patterned(4, 3))).with_repeat(ImageRepeat::Both))),
      ("an image repeating down", fill(rect(), Fill::image(Arc::new(patterned(5, 2))).with_repeat(ImageRepeat::Y))),
      ("an image repeating across", fill(rect(), Fill::image(Arc::new(patterned(3, 4))).with_repeat(ImageRepeat::X))),
      ("a repeating image and a mask", fill(rect(), Fill::image(Arc::new(patterned(4, 2))).with_repeat(ImageRepeat::Y)).with_mask(Image::new_from_pixels(30, 20, (0..30 * 20).flat_map(|i| [(i * 3 % 256) as u8, 0, 0, 255]).collect::<Vec<u8>>(), Channels::RGBA))),
      ("a moved fill", fill(circle(), blue).with_position((3.0, 7.0))),
    ];
    for (name, fill_area) in &fills {
      let mut expected = photo(width, height);
      fill_area.apply_on_cpu(&mut expected);
      let mut made = photo(width, height);
      assert!(made.draw_fill_on_gpu(fill_area), "{name}: the GPU did not take the fill");
      assert!(made.deferred().is_some(), "{name}: with a GPU the result is not made until it is read");
      let worst = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).max().unwrap_or(0);
      let first = made.rgba().chunks_exact(4).zip(expected.rgba().chunks_exact(4)).position(|(a, e)| a.iter().zip(e).any(|(a, e)| a.abs_diff(*e) > 3));
      let seen = first.map(|i| (i as u32 % width, i as u32 / width, made.rgba()[i * 4..i * 4 + 4].to_vec(), expected.rgba()[i * 4..i * 4 + 4].to_vec()));
      assert!(worst <= 3, "{name}: the GPU differs from the CPU by {worst}; first at (x, y, gpu, cpu) {seen:?}");
    }
    // A shape this large has more points than the shader takes, so it is simplified first. It must still be drawn by the GPU, and
    // agree with the CPU except for the odd sample on its edge.
    let big = fill(Area::circle((130.0, 130.0), 120.0), red);
    assert!(big.area.path.flatten(0.5).len() > MAX_FILL_POINTS, "the shape is not large enough to need simplifying");
    assert!(big.gpu_passes().is_some(), "a large shape was left to the CPU");
    let mut expected = photo(260, 260);
    big.apply_on_cpu(&mut expected);
    let mut made = photo(260, 260);
    assert!(made.draw_fill_on_gpu(&big), "a large shape was not taken by the GPU");
    assert!(made.deferred().is_some(), "a large shape was not made by the GPU");
    let differences: Vec<u8> = made.rgba().iter().zip(expected.rgba()).map(|(a, e)| a.abs_diff(*e)).collect();
    let (worst, wrong) = (differences.iter().max().copied().unwrap_or(0), differences.iter().filter(|difference| **difference > 3).count());
    assert!(worst <= 40 && wrong * 500 <= differences.len(), "a large shape differs from the CPU by {worst} in {wrong} bytes");

    assert!(RUNS.load(Ordering::SeqCst) >= fills.len(), "the GPU did not run the shader for every fill");
    abra_core::image::gpu::clear_gpu_provider();
    Ok(())
  }

  #[test]
  fn a_shape_with_too_many_points_is_left_to_the_cpu() {
    let points: Vec<[f32; 2]> = (0..=MAX_FILL_POINTS).map(|i| [i as f32, (i % 2) as f32 * 5.0]).collect();
    let jagged = fill(Area::from_points(&points), Color::from_rgba(0, 0, 0, 255));
    assert!(jagged.gpu_passes().is_none());
  }

  #[test]
  fn a_gradient_mixed_in_another_space_is_left_to_the_cpu_which_draws_it() {
    use abra_core::{ColorSpace, Gradient};

    let (blue, yellow) = (Color::from_rgb(0, 0, 255), Color::from_rgb(255, 255, 0));
    let area = Area::rect((0.0, 0.0), (16.0, 4.0));
    let in_srgb = fill(area.clone(), Gradient::from_to(blue, yellow));
    let in_oklab = fill(area.clone(), Gradient::from_to(blue, yellow).with_interpolation(ColorSpace::Oklab));
    // The shader mixes in sRGB, so the default gradient is its to draw and the other is not.
    assert!(in_srgb.gpu_passes().is_some());
    assert!(in_oklab.gpu_passes().is_none());
    // On the CPU the Oklab one has color in the middle where the sRGB one is a gray.
    let (srgb, oklab) = (in_srgb.to_image(), in_oklab.to_image());
    // How far apart the channels of the middle pixel are: a gray has none.
    let spread = |image: &Image| {
      let (r, g, b, _) = image.get_pixel(8, 2).unwrap();
      r.max(g).max(b) - r.min(g).min(b)
    };
    assert!(spread(&srgb) < 25, "sRGB is nearly a gray: {}", spread(&srgb));
    assert!(spread(&oklab) > 60, "Oklab has color: {}", spread(&oklab));
  }
}

// #[cfg(test)]
// mod tests {
//   use super::*;
//   use abra_core::{Area, Color};

//   #[test]
//   fn fill_with_feather_sets_alpha_near_edge() {
//     // 20x20 image, rectangle area with feather 4.
//     let area = Area::rect((2.0, 2.0), (16.0, 16.0)).with_feather(4);
//     let color = Color::from_rgba(0, 0, 0, 255);
//     let img = fill(area, color);
//     // Check center pixel alpha (should be fully opaque)
//     let (w, h) = img.dimensions::<u32>();
//     let cx = w / 2;
//     let cy = h / 2;
//     assert_eq!(img.get_pixel(cx, cy).unwrap().3, 255);
//     // Check a pixel near the edge (should be less than 255 if inside feather)
//     let near_edge = img.get_pixel(3, 3).unwrap().3;
//     assert!(near_edge < 255);
//   }
// }

#[cfg(test)]
mod fill_area_tests {
  use super::*;
  use abra_core::{Channels, Color};

  #[test]
  fn a_repeating_image_is_tiled_on_the_repeating_axis_only() {
    let tile = || Image::new_from_pixels(1, 2, vec![0, 0, 0, 255, 0, 0, 0, 0], Channels::RGBA);
    let column = |p_repeat: Option<ImageRepeat>| {
      let mut image = Image::new_from_color(3, 6, Color::from_rgba(0, 0, 0, 0));
      let source = Fill::image(Arc::new(tile()));
      let source = match p_repeat {
        Some(repeat) => source.with_repeat(repeat),
        None => source,
      };
      fill(Area::rect((0.0, 0.0), (3.0, 6.0)), source).apply_on_cpu(&mut image);
      (0..6).map(|y| image.get_pixel(1, y).unwrap().3).collect::<Vec<u8>>()
    };
    assert_eq!(column(None), vec![255, 0, 0, 0, 0, 0]);
    assert_eq!(column(Some(ImageRepeat::NoRepeat)), vec![255, 0, 0, 0, 0, 0]);
    assert_eq!(column(Some(ImageRepeat::Y)), vec![255, 0, 255, 0, 255, 0]);
    assert_eq!(column(Some(ImageRepeat::X)), vec![255, 0, 0, 0, 0, 0]);
  }

  #[test]
  fn a_checkerboard_fills_an_area_with_alternating_squares_from_its_top_left_corner() {
    let (light, dark) = ((200, 200, 200), (100, 100, 100));
    // Over a whole image, and over an area that starts away from the corner: the light square is at the area's own top left.
    for origin in [(0u32, 0u32), (3, 1)] {
      let mut image = Image::new_from_color(12, 9, Color::from_rgba(0, 0, 0, 0));
      fill(Area::rect((origin.0 as f32, origin.1 as f32), (7.0, 5.0)), Fill::checkerboard(2, light, dark)).apply_on_cpu(&mut image);
      for y in 0..5u32 {
        for x in 0..7u32 {
          let expected = if (x / 2 + y / 2) % 2 == 0 { (200, 200, 200, 255) } else { (100, 100, 100, 255) };
          assert_eq!(image.get_pixel(origin.0 + x, origin.1 + y), Some(expected), "origin {origin:?}, square at {x},{y}");
        }
      }
      assert_eq!(image.get_pixel(origin.0 + 7, origin.1).unwrap().3, 0, "nothing outside the area");
    }
  }

  #[test]
  fn a_fill_is_drawn_where_it_is_cheaper_and_a_target_with_gpu_work_waiting_keeps_it_there() {
    use abra_core::{Gradient, Path};
    let on = |size: (u32, u32), pending: bool| GpuState { size, pending };
    let disc = fill(Area::circle((100.0, 100.0), 36.0), Color::white());
    let whole = |side: f32| fill(Area::rect((0.0, 0.0), (side, side)), Gradient::from_to(Color::white(), Color::black()));
    // A small shape on a big picture is not worth a run of the GPU over the whole picture.
    assert!(!disc.cheaper_on_gpu(on((1000, 700), false)));
    assert!(!disc.cheaper_on_gpu(on((200, 200), false)));
    // A gradient over a whole picture is: the CPU works out a color for every pixel.
    assert!(whole(200.0).cheaper_on_gpu(on((200, 200), false)));
    assert!(whole(600.0).cheaper_on_gpu(on((600, 600), false)));
    // The same gradient over a small part of a big picture is not.
    assert!(!whole(100.0).cheaper_on_gpu(on((1000, 700), false)));
    // A line across a picture covers few of the pixels in its bounds.
    let mut across = Path::new();
    across.move_to((10.0, 190.0)).line_to((190.0, 10.0));
    assert!(!fill(across.stroke(4.0), Color::white()).cheaper_on_gpu(on((200, 200), false)));
    // A picture that is already waiting for the GPU takes anything there, because drawing on the CPU would make it run now.
    assert!(disc.cheaper_on_gpu(on((1000, 700), true)));
    // Only the part of the bounds on the target counts.
    assert!(!fill(Area::circle((-5000.0, -5000.0), 36.0), Color::white()).cheaper_on_gpu(on((200, 200), false)));
  }

  #[test]
  fn apply_draws_a_small_fill_now_and_leaves_a_target_with_nothing_waiting() {
    let mut image = Image::new_from_color(300, 200, Color::from_rgba(0, 0, 0, 255));
    fill(Area::circle((50.0, 50.0), 10.0), Color::white()).apply(&mut image);
    assert!(image.deferred().is_none(), "a small fill was left for the GPU");
    assert_eq!(image.get_pixel(50, 50), Some((255, 255, 255, 255)));
  }

  /// Where the GPU and the CPU each win, which is what the numbers in `cost` come from, and what `apply` does with them. Run with
  /// `cargo test -p drawing --lib timing_of_gpu_and_cpu_fills -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_gpu_and_cpu_fills() {
    use abra_core::{Gradient, Path};
    if gpu::GpuContext::new_default_blocking().is_err() {
      return;
    }
    gpu::register();
    let shapes: Vec<(&str, Box<dyn Fn() -> FillArea<'static>>)> = vec![
      ("small disc, solid", Box::new(|| fill(Area::circle((100.0, 100.0), 36.0), Color::white()))),
      ("thin diagonal line, solid", Box::new(|| {
        let mut path = Path::new();
        path.move_to((10.0, 190.0)).line_to((190.0, 10.0));
        fill(path.stroke(4.0), Color::white())
      })),
      ("half-size rect, solid", Box::new(|| fill(Area::rect((0.0, 0.0), (100.0, 100.0)), Color::white()))),
      ("half-size rect, gradient", Box::new(|| fill(Area::rect((0.0, 0.0), (100.0, 100.0)), Gradient::from_to(Color::white(), Color::black())))),
      ("whole rect, gradient", Box::new(|| fill(Area::rect((0.0, 0.0), (200.0, 200.0)), Gradient::from_to(Color::white(), Color::black())))),
    ];
    for (width, height) in [(200u32, 200u32), (1000, 700)] {
      for (name, make) in &shapes {
        // The shapes are described for a 200 x 200 target; on a bigger target they are the same shapes, smaller in proportion.
        let time = |how: u8| {
          let run = || {
            let mut image = Image::new(width, height);
            match how {
              0 => {
                image.draw_fill_on_gpu(&make());
              }
              1 => make().apply_on_cpu(&mut image),
              _ => make().apply(&mut image),
            }
            let _ = image.rgba().len();
          };
          for _ in 0..3 {
            run();
          }
          let started = std::time::Instant::now();
          for _ in 0..10 {
            run();
          }
          started.elapsed().as_secs_f64() * 100.0
        };
        println!("{width}x{height} {name}: gpu {:.2} ms, cpu {:.2} ms, apply {:.2} ms", time(0), time(1), time(2));
      }
    }
  }

  #[test]
  fn apply_draws_the_area_where_it_is() {
    let mut image = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    fill(Area::rect((4.0, 4.0), (2.0, 2.0)), Color::from_rgba(255, 0, 0, 255)).apply(&mut image);
    assert_eq!(image.get_pixel(4, 4), Some((255, 0, 0, 255)));
    assert_eq!(image.get_pixel(1, 1), Some((0, 0, 0, 255)));
  }

  #[test]
  fn with_position_moves_the_area() {
    let mut image = Image::new_from_color(8, 8, Color::from_rgba(0, 0, 0, 255));
    fill(Area::rect((4.0, 4.0), (2.0, 2.0)), Color::from_rgba(255, 0, 0, 255))
      .with_position((0.0, 0.0))
      .apply(&mut image);
    assert_eq!(image.get_pixel(0, 0), Some((255, 0, 0, 255)));
    assert_eq!(image.get_pixel(4, 4), Some((0, 0, 0, 255)));
  }

  #[test]
  fn to_image_is_the_size_of_the_bounds() {
    let image = fill(Area::rect((4.0, 4.0), (3.0, 2.0)), Color::from_rgba(255, 0, 0, 255)).to_image();
    assert_eq!(image.dimensions::<u32>(), (3, 2));
  }

  #[test]
  fn with_mask_multiplies_fill_alpha() {
    let mask = Image::new_from_pixels(
      3,
      3,
      [255u8, 128, 0].repeat(3).into_iter().flat_map(|value| [value, value, value, 255]).collect::<Vec<u8>>(),
      Channels::RGBA,
    );
    let filled = fill(Area::rect((0.0, 0.0), (3.0, 3.0)), Color::from_rgba(255, 0, 0, 255)).with_mask(mask).to_image();

    assert_eq!(filled.get_pixel(0, 1).unwrap().3, 255);
    assert_eq!(filled.get_pixel(1, 1).unwrap().3, 128);
    assert_eq!(filled.get_pixel(2, 1).unwrap().3, 0);
  }
}
