use abra_core::image::effect::{Effect, deferred};
use abra_core::{Area, Fill, Image, LineCap, LineJoin, Path, Performance, PointF};

use crate::{
  CoverageMask, PolygonCoverage, Rasterizer, SampleGrid, Shader, Sprite, SpriteTrail, TrailPoint,
  brush::brush::Brush,
  performance::{DrawingAlgorithm, ResolvedPerformance, configured_performance},
  shader_from_fill_with_path, sprites,
  shaders::{brush_dabs_shader::BrushDabsShader, brush_shader::BrushShader, stroke_brush_shader::StrokeBrushShader},
};

/// The brush's dab as a sprite of the stamping engine, or nothing when the engine cannot draw it: a brush that is not a solid color, or
/// whose tip is not the default round one.
fn dab_sprite(p_x: f32, p_y: f32, p_brush: &Brush) -> Option<Sprite> {
  let Fill::Solid(color) = p_brush.color() else {
    return None;
  };
  let path = &p_brush.area().path;
  let (left, top, right, bottom) = path.bounds().edges::<f32>();
  let (center_x, center_y, radius) = ((left + right) / 2.0, (top + bottom) / 2.0, (right - left) / 2.0);
  // The default tip is a circle of radius 5 around the origin, which the brush's size scales to its own.
  let round = (radius - 5.0).abs() < 0.05
    && (bottom - top - 2.0 * radius).abs() < 0.05
    && center_x.abs() < 0.05
    && center_y.abs() < 0.05
    && path.flatten(0.25).iter().all(|point| ((point.x - center_x).hypot(point.y - center_y) - radius).abs() < 0.1 * radius);
  if !round {
    return None;
  }
  let (red, green, blue, alpha) = color.rgba();
  Some(
    Sprite::new(p_x, p_y, p_brush.size() / 2.0)
      .with_hardness(p_brush.hardness())
      .with_color((red, green, blue))
      .with_opacity(p_brush.opacity().opacity_at(0.0) * alpha as f32 / 255.0)
      .with_blend(p_brush.blend()),
  )
}

/// The brush's stroke along a path as a trail of the stamping engine, or nothing when the engine cannot draw it: a brush that is not a
/// solid color or a gradient that runs along the stroke's own path, or a path with no length.
fn stroke_trail(p_path: &Path, p_brush: &Brush) -> Option<SpriteTrail> {
  let gradient = match p_brush.color() {
    Fill::Solid(_) => None,
    Fill::Gradient(gradient) if gradient.direction().is_some_and(|direction| direction.flatten(1.0) == p_path.flatten(1.0)) => Some(gradient),
    _ => return None,
  };
  let (width, opacity) = (p_brush.width(), p_brush.opacity());
  let points = p_path.flatten((width.widest() * 0.01).clamp(0.1, 0.5));
  if points.len() < 2 {
    return None;
  }
  // How far along the path each point is, by length: `0` at the first and `1` at the last, where the width and opacity are read.
  let mut lengths = vec![0.0f32; points.len()];
  for at in 1..points.len() {
    lengths[at] = lengths[at - 1] + ((points[at].x - points[at - 1].x).powi(2) + (points[at].y - points[at - 1].y).powi(2)).sqrt();
  }
  let total = *lengths.last().unwrap();
  if total <= 0.0 {
    return None;
  }
  let solid = match p_brush.color() {
    Fill::Solid(color) => Some(color.rgba()),
    _ => None,
  };
  let trail = points.iter().zip(&lengths).map(|(point, length)| {
    let along = length / total;
    let (red, green, blue, alpha) = match (&solid, &gradient) {
      (Some(color), _) => *color,
      (None, Some(gradient)) => gradient.color_at(along).rgba(),
      (None, None) => unreachable!("a stroke has a solid color or a gradient"),
    };
    TrailPoint::new(point.x, point.y, width.width_at(along)).with_opacity(opacity.opacity_at(along) * alpha as f32 / 255.0).with_color((red, green, blue))
  });
  Some(SpriteTrail::new(trail.collect()).with_hardness(p_brush.hardness()).with_blend(p_brush.blend()))
}

/// Unified drawing context for an image.
pub struct Painter<'a> {
  image: &'a mut Image,
  performance: ResolvedPerformance,
}

impl<'a> Painter<'a> {
  /// Creates a new painter for the given image.
  pub fn new(p_image: &'a mut Image) -> Self {
    Self::with_performance(p_image, configured_performance())
  }

  /// Creates a painter with an explicit rendering profile.
  pub fn with_performance(p_image: &'a mut Image, p_performance: Performance<DrawingAlgorithm>) -> Self {
    Painter {
      image: p_image,
      performance: ResolvedPerformance::from_performance(p_performance),
    }
  }

  /// Whether a dab, stroke or fill that the stamping engine can draw is drawn by it. The `Rasterizer` algorithm (the quality profile) asks for the
  /// general supersampled rasterizer for every shape, so it is not.
  fn uses_engine(&self) -> bool {
    self.performance.algorithm != DrawingAlgorithm::Rasterizer
  }

  /// Paints a single brush dab at a specific position.
  /// - `p_x`: The x-coordinate to paint at.
  /// - `p_y`: The y-coordinate to paint at.
  /// - `p_brush`: The brush to use for painting.
  pub fn dab_brush(&mut self, p_x: f32, p_y: f32, p_brush: &Brush) {
    // A round dab of a solid color is a sprite, which the GPU draws when there is one and the image can wait for it.
    if self.uses_engine()
      && let Some(dot) = dab_sprite(p_x, p_y, p_brush)
    {
      sprites().with_sprites(vec![dot]).apply(deferred(&mut *self.image));
      return;
    }
    let size = p_brush.size();
    let area = p_brush.area();
    let fill = p_brush.color();

    let scale_factor = size / 10.0;

    let tolerance = (size * 0.01).max(0.5).min(8.0);
    let flattened: Vec<PointF> = area
      .path
      .flatten(tolerance)
      .into_iter()
      .map(|p| PointF::new(p.x * scale_factor + p_x, p.y * scale_factor + p_y))
      .collect();

    let coverage = PolygonCoverage::new(flattened);
    // Wrap inner shader with BrushShader to apply alpha falloff based on hardness
    // Build a default gradient path spanning the dab horizontally so
    // linear gradients without explicit direction are visible.
    let dab_path = Path::line((p_x - size / 2.0, p_y), (p_x + size / 2.0, p_y));
    let inner_shader = shader_from_fill_with_path(fill.clone(), Some(dab_path));
    let max_distance = size / 2.0;
    let shader: Box<dyn Shader + Send + Sync> =
      Box::new(BrushShader::new(inner_shader, p_x, p_y, max_distance, p_brush.hardness()));
    let compositor = p_brush.blend();
    let sample_grid = SampleGrid::from_aa_level(self.performance.anti_aliasing);
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);

    rasterizer.rasterize(self.image);
  }

  /// Strokes a path with a brush by converting it into a stroked area
  /// and filling that area in a single rasterization pass.
  /// - `p_path`: The path to stroke.
  /// - `p_brush`: The brush to use for stroking.
  pub fn stroke_with_brush(&mut self, p_path: &Path, p_brush: &Brush) {
    let width = p_brush.width();
    let opacity = p_brush.opacity();

    // A stroke of a solid color, or of a gradient that runs along it, is a trail of the stamping engine, which the GPU draws when there
    // is one and the image can wait for it. Other fills are drawn by the rasterizer below.
    if self.uses_engine()
      && let Some(trail) = stroke_trail(p_path, p_brush)
    {
      sprites().with_trails(vec![trail]).apply(deferred(&mut *self.image));
      return;
    }

    // Convert open path into an area and then create a stroked outline
    // using round joins for smooth corners.
    let stroke_path = p_path.stroke(width.clone()).with_join(LineJoin::Round).with_cap(LineCap::Round).to_path();
    let stroke_area: Area = stroke_path.into();

    // Build flattened polygon coverage for the stroked area
    let tolerance = (width.widest() * 0.01).max(0.5).min(8.0);
    let flattened: Vec<PointF> =
      stroke_area.path.flatten(tolerance).into_iter().map(|p| PointF::new(p.x, p.y)).collect();

    let coverage = PolygonCoverage::new(flattened);

    // Create inner shader from fill and wrap in StrokeBrushShader to compute falloff from path centerline
    // For stroke brushes, prefer the stroke path as the gradient direction so
    // gradients are oriented along the stroke centerline.
    let inner_shader = shader_from_fill_with_path(p_brush.color().clone(), Some(p_path.clone()));
    // Path stroke shading falloff radius is half the width where the stroke is
    let mut stroke_shader = StrokeBrushShader::new(inner_shader, p_path, width, opacity, p_brush.hardness());
    // A gradient that runs along this stroke's own path is read where the stroke shader has already found the closest place on it.
    if let abra_core::Fill::Gradient(gradient) = p_brush.color()
      && gradient.direction().is_some_and(|direction| direction.flatten(1.0) == p_path.flatten(1.0))
    {
      stroke_shader = stroke_shader.with_gradient_along_path(gradient.clone().into_owned());
    }
    let shader: Box<dyn Shader + Send + Sync> = Box::new(stroke_shader);

    let compositor = p_brush.blend();
    let sample_grid = SampleGrid::from_aa_level(self.performance.anti_aliasing);
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);

    rasterizer.rasterize(self.image);
  }

  pub fn fill_area_with_brush(&mut self, p_area: &Area, p_brush: &Brush) {
    // Fill the polygonal area by repeatedly painting brush "dabs" across the
    // area instead of using a single-shader center.  The previous behavior
    // used a BrushShader centered at (0,0) which only affected a small
    // region near the origin resulting in a single point being drawn.
    //
    // We compute a grid of brush centers across the area's bounding box and
    // paint a dab at each center if it falls inside the polygon coverage.
    let tolerance = 0.5;
    let flattened: Vec<PointF> = p_area.path.flatten(tolerance).into_iter().map(|p| PointF::new(p.x, p.y)).collect();

    let coverage = PolygonCoverage::new(flattened);
    if let Some((min_x, min_y, max_x, max_y)) = coverage.bounds() {
      let radius = p_brush.size() / 2.0;
      if radius <= 0.0 {
        return;
      }

      let stride = (radius / 3.0).max(1.0);
      let mut centers: Vec<PointF> = Vec::new();
      let diag = radius * std::f32::consts::FRAC_1_SQRT_2;

      let mut y = (min_y - radius).floor();
      while y <= (max_y + radius).ceil() {
        let mut x = (min_x - radius).floor();
        while x <= (max_x + radius).ceil() {
          if coverage.contains(x, y)
            || coverage.contains(x + radius, y)
            || coverage.contains(x - radius, y)
            || coverage.contains(x, y + radius)
            || coverage.contains(x, y - radius)
            || coverage.contains(x + diag, y + diag)
            || coverage.contains(x + diag, y - diag)
            || coverage.contains(x - diag, y + diag)
            || coverage.contains(x - diag, y - diag)
          {
            centers.push(PointF::new(x, y));
          }
          x += stride;
        }
        y += stride;
      }

      // Round dabs of a solid color are sprites of the stamping engine, limited to the area, which the GPU draws when there is one.
      if self.uses_engine() && !centers.is_empty() && dab_sprite(0.0, 0.0, p_brush).is_some() {
        let dabs: Vec<Sprite> = centers.iter().filter_map(|center| dab_sprite(center.x, center.y, p_brush)).collect();
        sprites().with_sprites(dabs).with_area(p_area.clone()).apply(deferred(&mut *self.image));
        return;
      }

      if !centers.is_empty() {
        // Use a gradient path covering the area so gradients without explicit
        // direction are visible across the whole area.
        let bounds_path = Path::line((min_x, min_y), (max_x, min_y));
        let inner_shader = shader_from_fill_with_path(p_brush.color().clone(), Some(bounds_path));
        let shader: Box<dyn Shader + Send + Sync> =
          Box::new(BrushDabsShader::new(inner_shader, centers, radius, p_brush.hardness()));
        let compositor = p_brush.blend();
        let sample_grid = SampleGrid::from_aa_level(self.performance.anti_aliasing);
        let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);
        rasterizer.rasterize(self.image);
      }
    }
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Blend, Brush};
  use abra_core::BlendMode;
  use abra_core::{Color, Image, Path, StrokeOpacity, StrokeWidth};

  #[test]
  fn stroke_ellipse_fastpath_works() {
    let mut img = Image::new(1200, 1200);
    let path = Path::from(Area::ellipse((600.0, 600.0), (2.0 * 400.0, 2.0 * 400.0)));
    let black = Color::black();
    let brush = Brush::new().with_size(40).with_color(&black).with_hardness(1.0);
    // Should take the fast SDF path and write pixels
    Painter::new(&mut img).stroke_with_brush(&path, &brush);
    // Verify that pixels near the ring have non-zero alpha
    let rgba = img.rgba();
    let idx = ((600 * 1200 + 1000) as usize) * 4 + 3;
    assert!(rgba[idx] != 0, "expected alpha written by fast-path stroke");
  }

  #[test]
  fn a_brush_blend_decides_how_a_stroke_meets_what_is_under_it() {
    let line = || {
      let mut path = Path::new();
      path.move_to((10.0, 20.0)).line_to((90.0, 20.0));
      path
    };
    let gray = || Image::new_from_color(100, 40, Color::from_rgb(100, 100, 100));
    let white = Color::white();
    let brush = || Brush::new().with_size(10).with_color(&white).with_hardness(1.0).with_opacity(0.5);
    let middle = |image: &Image| image.get_pixel(50, 20).unwrap();

    let (mut normal, mut additive, mut multiply) = (gray(), gray(), gray());
    Painter::new(&mut normal).stroke_with_brush(&line(), &brush());
    Painter::new(&mut additive).stroke_with_brush(&line(), &brush().with_blend(Blend::Additive));
    Painter::new(&mut multiply).stroke_with_brush(&line(), &brush().with_blend(BlendMode::Multiply));
    // Half opaque white: painted over, it is the mean of 100 and 255; added, it is 100 plus half of 255; multiplied, it changes nothing.
    assert!((176..=180).contains(&middle(&normal).0), "normal gave {}", middle(&normal).0);
    assert!((225..=230).contains(&middle(&additive).0), "additive gave {}", middle(&additive).0);
    assert!((99..=101).contains(&middle(&multiply).0), "multiply gave {}", middle(&multiply).0);
    // Away from the stroke nothing changed.
    assert_eq!(additive.get_pixel(50, 2).unwrap().0, 100);
  }

  /// How many pixels of a column of the image have anything painted on them.
  fn painted_in_column(p_image: &Image, p_x: u32) -> usize {
    let (width, height) = p_image.dimensions::<u32>();
    let rgba = p_image.rgba();
    (0..height).filter(|y| rgba[((y * width + p_x) as usize) * 4 + 3] > 127).count()
  }

  #[test]
  fn a_brush_with_a_width_that_tapers_paints_a_thinner_line_toward_the_end() {
    let mut img = Image::new(100, 40);
    let mut path = Path::new();
    path.move_to((10.0, 20.0)).line_to((90.0, 20.0));
    let brush = Brush::new().with_size(20).with_width(StrokeWidth::from_to(20.0, 2.0)).with_color(&Color::black()).with_hardness(1.0);
    Painter::new(&mut img).stroke_with_brush(&path, &brush);
    let (near_start, middle, near_end) = (painted_in_column(&img, 15), painted_in_column(&img, 50), painted_in_column(&img, 85));
    // The width at those places is 18.9, 11 and 3.1, and a hard stroke is solid over all of its width: about 19, 11 and 3 rows.
    assert!(near_start > middle && middle > near_end, "{near_start} {middle} {near_end}");
    assert!((17..=20).contains(&near_start) && (2..=4).contains(&near_end), "{near_start} {near_end}");
  }

  #[test]
  fn a_stroke_is_as_opaque_as_the_brush_and_fades_when_the_opacity_does() {
    let mut path = Path::new();
    path.move_to((10.0, 20.0)).line_to((90.0, 20.0));
    let alpha_at = |p_image: &Image, p_x: usize| p_image.rgba()[(20 * 100 + p_x) * 4 + 3] as i32;
    let mut half = Image::new(100, 40);
    Painter::new(&mut half).stroke_with_brush(&path, &Brush::new().with_size(12).with_color(&Color::black()).with_hardness(1.0).with_opacity(0.5));
    assert!((alpha_at(&half, 50) - 128).abs() <= 2, "{}", alpha_at(&half, 50));
    let mut faded = Image::new(100, 40);
    Painter::new(&mut faded).stroke_with_brush(&path, &Brush::new().with_size(12).with_color(&Color::black()).with_hardness(1.0).with_opacity(StrokeOpacity::from_to(1.0, 0.0)));
    let (start, middle, end) = (alpha_at(&faded, 14), alpha_at(&faded, 50), alpha_at(&faded, 86));
    assert!(start > 220 && (110..=145).contains(&middle) && end < 40, "{start} {middle} {end}");
  }

  #[test]
  fn setting_a_size_after_a_width_replaces_it() {
    let brush = Brush::new().with_width(StrokeWidth::from_to(8.0, 0.0)).with_size(6);
    assert_eq!(brush.width().constant_width(), Some(6.0));
    assert_eq!(Brush::new().width().constant_width(), Some(5.0));
  }

  #[test]
  fn benchmark_fast_vs_raster() {
    use std::time::Instant;
    let mut img_fast = Image::new(1200, 1200);
    let mut img_raster = img_fast.clone();
    let path_fast = Path::from(Area::ellipse((600.0, 600.0), (2.0 * 500.0, 2.0 * 500.0)));
    // Create a perturbed path by flattening and slightly jittering points to defeat ellipse detection
    let mut pts = path_fast.flatten(0.5);
    for (i, p) in pts.iter_mut().enumerate() {
      if i % 10 == 0 {
        p.x += 0.3; // small perturbation
      }
    }
    let mut perturbed = Path::new();
    if let Some(first) = pts.first() {
      perturbed.move_to(*first);
      for p in pts.iter().skip(1) {
        perturbed.line_to(*p);
      }
    }
    let black = Color::black();
    let brush = Brush::new().with_size(1000).with_color(&black).with_hardness(1.0);

    let t0 = Instant::now();
    Painter::new(&mut img_fast).stroke_with_brush(&path_fast, &brush);
    let fast_ms = t0.elapsed().as_millis();

    let t1 = Instant::now();
    Painter::new(&mut img_raster).stroke_with_brush(&perturbed, &brush);
    let raster_ms = t1.elapsed().as_millis();

    // Print results for manual inspection if running with --nocapture
    println!("fast_ms = {} ms, raster_ms = {} ms", fast_ms, raster_ms);

    // Expect fast path to be at least faster than raster by factor 2 in most cases
    // assert!(fast_ms * 2 < raster_ms || raster_ms > 1000, "fast path not significantly faster: {} vs {}", fast_ms, raster_ms);
  }

  #[test]
  fn reproduce_cursor_slow_case() {
    use std::time::Instant;
    let size = 1000.0f32;
    let mut cursor = Image::new((size + 2.0) as u32, (size + 2.0) as u32);
    let path = Path::from(Area::ellipse(((size / 2.0) + 1.0, (size / 2.0) + 1.0), (size, size)));
    let black = Color::black();
    let brush = Brush::new().with_size(4).with_color(&black).with_hardness(1.0);

    let t0 = Instant::now();
    Painter::new(&mut cursor).stroke_with_brush(&path, &brush);
    let elapsed = t0.elapsed();
    println!("cursor stroke time: {:?}", elapsed);

    // Ensure some pixels were written
    let rgba = cursor.rgba();
    let idx = (((size as u32 / 2) * (size as u32 + 2) + (size as u32)) as usize) * 4 + 3;
    assert!(rgba[idx] != 0, "expected alpha written by cursor stroke");
  }

  /// A stroke and a dab are drawn by the GPU when there is one, and give the CPU's pixels. The image waits for the GPU, so it is not made until
  /// its pixels are read.
  #[test]
  fn a_stroke_and_a_dab_are_drawn_by_the_gpu_when_there_is_one() -> anyhow::Result<()> {
    use abra_core::image::gpu::{GPU_PROVIDER_LOCK, clear_gpu_provider};
    use abra_core::{Channels, Performance};

    let (width, height) = (97u32, 61u32);
    let photo = || Image::new_from_pixels(width, height, [90u8, 120, 150, 255].repeat((width * height) as usize), Channels::RGBA);
    let mut path = Path::new();
    path.move_to((8.0, 50.0)).quad_to((40.0, 2.0), (90.0, 40.0));
    let stroke = Brush::new().with_size(9).with_color(&Color::red()).with_hardness(0.6).with_opacity(StrokeOpacity::from_to(1.0, 0.4));
    let dab = Brush::new().with_size(14).with_color(&Color::white()).with_hardness(0.3);
    let draw = |image: &mut Image| {
      let mut painter = Painter::new(image);
      painter.stroke_with_brush(&path, &stroke);
      painter.dab_brush(30.5, 30.5, &dab);
    };

    let _guard = GPU_PROVIDER_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    clear_gpu_provider();
    let mut on_cpu = photo();
    draw(&mut on_cpu);

    gpu::GpuContext::new_default_blocking()?;
    gpu::register();
    let mut on_gpu = photo();
    draw(&mut on_gpu);
    assert!(on_gpu.deferred().is_some(), "with a GPU the drawing waits until the pixels are read");
    let worst = on_gpu.rgba().iter().zip(on_cpu.rgba()).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    assert!(worst <= 2, "the GPU differs from the CPU by {worst}");
    assert_ne!(on_cpu.rgba(), photo().rgba(), "something was drawn");
    // The Rasterizer algorithm asks for the supersampled rasterizer, which the CPU runs at once.
    let mut rasterized = photo();
    {
      let mut painter = Painter::with_performance(&mut rasterized, Performance::Algorithm(DrawingAlgorithm::Rasterizer));
      painter.stroke_with_brush(&path, &stroke);
      painter.dab_brush(30.5, 30.5, &dab);
    }
    assert!(rasterized.deferred().is_none(), "the rasterizer draws at once");
    assert_ne!(rasterized.rgba(), photo().rgba(), "something was drawn");
    clear_gpu_provider();
    Ok(())
  }

  #[test]
  fn an_area_is_filled_with_dabs_and_nothing_outside_it_is_touched() {
    let mut image = Image::new(60, 40);
    let brush = Brush::new().with_size(8).with_color(&Color::red()).with_hardness(0.5);
    Painter::new(&mut image).fill_area_with_brush(&Area::rect((10.0, 10.0), (30.0, 20.0)), &brush);
    let alpha = |x: u32, y: u32| image.get_pixel(x, y).unwrap().3;
    assert!(alpha(25, 20) > 240, "the middle of the area is filled: {}", alpha(25, 20));
    assert_eq!(alpha(5, 20), 0, "left of the area");
    assert_eq!(alpha(25, 5), 0, "above the area");
    assert_eq!(alpha(50, 20), 0, "right of the area");
  }

  #[test]
  fn a_brush_can_be_a_fraction_of_a_pixel_across() {
    assert_eq!(Brush::new().with_size(2.5).size(), 2.5);
    assert_eq!(Brush::new().with_size(7).size(), 7.0);
    let mut image = Image::new(20, 20);
    Painter::new(&mut image).dab_brush(10.5, 10.5, &Brush::new().with_size(2.5).with_hardness(1.0).with_color(&Color::white()));
    assert_eq!(image.get_pixel(10, 10).unwrap().3, 255);
    assert_eq!(image.get_pixel(13, 10).unwrap().3, 0);
  }
}
