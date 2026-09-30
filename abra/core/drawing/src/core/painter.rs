use abra_core::{Area, Image, LineCap, LineJoin, Path, Performance, PointF};

use crate::{
  CoverageMask, PolygonCoverage, Rasterizer, SampleGrid, Shader, SourceOverCompositor,
  brush::brush::Brush,
  performance::{DrawingAlgorithm, ResolvedPerformance, configured_performance},
  shader_from_fill_with_path,
  shaders::{brush_dabs_shader::BrushDabsShader, brush_shader::BrushShader, stroke_brush_shader::StrokeBrushShader},
};

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

  /// Paints a single brush dab at a specific position.
  /// - `p_x`: The x-coordinate to paint at.
  /// - `p_y`: The y-coordinate to paint at.
  /// - `p_brush`: The brush to use for painting.
  pub fn dab_brush(&mut self, p_x: f32, p_y: f32, p_brush: &Brush) {
    let size = p_brush.size() as f32;
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
    let compositor = SourceOverCompositor;
    let sample_grid = SampleGrid::from_aa_level(self.performance.anti_aliasing);
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);

    rasterizer.rasterize(self.image);
  }

  /// Strokes a path with a brush by converting it into a stroked area
  /// and filling that area in a single rasterization pass.
  /// - `p_path`: The path to stroke.
  /// - `p_brush`: The brush to use for stroking.
  pub fn stroke_with_brush(&mut self, p_path: &Path, p_brush: &Brush) {
    let width = p_brush.size() as f32;

    // Fast-path: if the original path is an ellipse and the brush is a solid color,
    // use the SDF-based rasterizer for a much faster stroke of large circular shapes.
    fn detect_ellipse(p_path: &Path) -> Option<(f32, f32, f32, f32)> {
      let (min_x, min_y, max_x, max_y) = p_path.bounds().edges::<f32>();
      let rx = (max_x - min_x) / 2.0;
      let ry = (max_y - min_y) / 2.0;
      if rx <= 0.0 || ry <= 0.0 {
        return None;
      }
      let cx = (min_x + max_x) / 2.0;
      let cy = (min_y + max_y) / 2.0;
      let tol = 0.06; // allow some small deviation
      for p in p_path.flatten(1.0) {
        let dx = (p.x - cx) / rx;
        let dy = (p.y - cy) / ry;
        let r = (dx * dx + dy * dy).sqrt();
        if (r - 1.0).abs() > tol {
          return None;
        }
      }
      Some((cx, cy, rx, ry))
    }

    if self.performance.algorithm != DrawingAlgorithm::Rasterizer
      && let Some((cx, cy, rx, ry)) = detect_ellipse(p_path)
    {
      // Only fast-path for solid-color brushes
      match p_brush.color() {
        abra_core::Fill::Solid(_color) => {
          // Draw stroke using SDF ellipse rasterizer
          crate::draw_ellipse_stroke(self.image, cx, cy, rx, ry, p_brush);
          return;
        }
        _ => {}
      }
    }

    // Convert open path into an area and then create a stroked outline
    // using round joins for smooth corners.
    let stroke_path = p_path.stroke(width).with_join(LineJoin::Round).with_cap(LineCap::Round).to_path();
    let stroke_area: Area = stroke_path.into();

    // Build flattened polygon coverage for the stroked area
    let tolerance = (width * 0.01).max(0.5).min(8.0);
    let flattened: Vec<PointF> =
      stroke_area.path.flatten(tolerance).into_iter().map(|p| PointF::new(p.x, p.y)).collect();

    let coverage = PolygonCoverage::new(flattened);

    // Create inner shader from fill and wrap in StrokeBrushShader to compute falloff from path centerline
    // For stroke brushes, prefer the stroke path as the gradient direction so
    // gradients are oriented along the stroke centerline.
    let inner_shader = shader_from_fill_with_path(p_brush.color().clone(), Some(p_path.clone()));
    // Path stroke shading falloff radius is (width / 2)
    let max_distance = width / 2.0;
    let shader: Box<dyn Shader + Send + Sync> =
      Box::new(StrokeBrushShader::new(inner_shader, p_path, max_distance, p_brush.hardness()));

    let compositor = SourceOverCompositor;
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
      let radius = p_brush.size() as f32 / 2.0;
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

      if !centers.is_empty() {
        // Use a gradient path covering the area so gradients without explicit
        // direction are visible across the whole area.
        let bounds_path = Path::line((min_x, min_y), (max_x, min_y));
        let inner_shader = shader_from_fill_with_path(p_brush.color().clone(), Some(bounds_path));
        let shader: Box<dyn Shader + Send + Sync> =
          Box::new(BrushDabsShader::new(inner_shader, centers, radius, p_brush.hardness()));
        let compositor = SourceOverCompositor;
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
  use crate::Brush;
  use abra_core::{Color, Image, Path};

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
}
