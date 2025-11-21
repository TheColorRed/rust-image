use core::{Area, Image, LineCap, LineJoin, Path, PointF};

use crate::{
  CoverageMask, PolygonCoverage, Rasterizer, SampleGrid, Shader, SourceOverCompositor,
  brush::brush::Brush,
  shader_from_fill_with_path,
  shaders::{brush_dabs_shader::BrushDabsShader, brush_shader::BrushShader, stroke_brush_shader::StrokeBrushShader},
};

/// Unified drawing context for an image.
pub struct Painter<'a> {
  image: &'a mut Image,
}

impl<'a> Painter<'a> {
  /// Creates a new painter for the given image.
  pub fn new(image: &'a mut Image) -> Self {
    Painter { image }
  }

  /// Paints a single brush dab at a specific position.
  /// - `x`: The x-coordinate to paint at.
  /// - `y`: The y-coordinate to paint at.
  /// - `brush`: The brush to use for painting.
  pub fn dab_brush(&mut self, x: f32, y: f32, brush: &Brush) {
    let size = brush.size() as f32;
    let area = brush.area();
    let fill = brush.color();

    let scale_factor = size / 10.0;

    let tolerance = 0.5;
    let flattened: Vec<PointF> = area
      .path
      .flatten(tolerance)
      .into_iter()
      .map(|p| PointF::new(p.x * scale_factor + x, p.y * scale_factor + y))
      .collect();

    let coverage = PolygonCoverage::new(flattened);
    // Wrap inner shader with BrushShader to apply alpha falloff based on hardness
    // Build a default gradient path spanning the dab horizontally so
    // linear gradients without explicit direction are visible.
    let dab_path = Path::line((x - size / 2.0, y), (x + size / 2.0, y));
    let inner_shader = shader_from_fill_with_path(fill.clone(), Some(dab_path));
    let max_distance = size / 2.0;
    let shader: Box<dyn Shader + Send + Sync> =
      Box::new(BrushShader::new(inner_shader, x, y, max_distance, brush.hardness()));
    let compositor = SourceOverCompositor;
    let sample_grid = SampleGrid::from_aa_level(2);
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);

    rasterizer.rasterize(self.image);
  }

  /// Strokes a path with a brush by converting it into a stroked area
  /// and filling that area in a single rasterization pass.
  /// - `path`: The path to stroke.
  /// - `brush`: The brush to use for stroking.
  pub fn stroke_with_brush(&mut self, path: &Path, brush: &Brush) {
    let width = brush.size() as f32;

    // Convert open path into an area and then create a stroked outline
    // using round joins for smooth corners.
    let stroke_path = path.stroke(width, LineJoin::Round, LineCap::Round);
    let stroke_area: Area = stroke_path.into();

    // Build flattened polygon coverage for the stroked area
    let tolerance = 0.5;
    let flattened: Vec<PointF> = stroke_area
      .path
      .flatten(tolerance)
      .into_iter()
      .map(|p| PointF::new(p.x, p.y))
      .collect();

    let coverage = PolygonCoverage::new(flattened);

    // Create inner shader from fill and wrap in StrokeBrushShader to compute falloff from path centerline
    // For stroke brushes, prefer the stroke path as the gradient direction so
    // gradients are oriented along the stroke centerline.
    let inner_shader = shader_from_fill_with_path(brush.color().clone(), Some(path.clone()));
    // Path stroke shading falloff radius is (width / 2)
    let max_distance = width / 2.0;
    let shader: Box<dyn Shader + Send + Sync> =
      Box::new(StrokeBrushShader::new(inner_shader, path.clone(), max_distance, brush.hardness()));

    let compositor = SourceOverCompositor;
    let sample_grid = SampleGrid::from_aa_level(2);
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);

    rasterizer.rasterize(self.image);
  }

  pub fn fill_area_with_brush(&mut self, area: &Area, brush: &Brush) {
    // Fill the polygonal area by repeatedly painting brush "dabs" across the
    // area instead of using a single-shader center.  The previous behavior
    // used a BrushShader centered at (0,0) which only affected a small
    // region near the origin resulting in a single point being drawn.
    //
    // We compute a grid of brush centers across the area's bounding box and
    // paint a dab at each center if it falls inside the polygon coverage.
    let tolerance = 0.5;
    let flattened: Vec<PointF> = area
      .path
      .flatten(tolerance)
      .into_iter()
      .map(|p| PointF::new(p.x, p.y))
      .collect();

    let coverage = PolygonCoverage::new(flattened);
    if let Some((min_x, min_y, max_x, max_y)) = coverage.bounds() {
      // Build a grid of dab centers across the polygon and collect
      // those inside the coverage. We will shade all centers in a
      // single rasterization pass using `BrushDabsShader`.

      // Compute effective size used for generating dabs. If the user chose a very
      // small brush the rasterization cost gets very high due to dense centers.
      // To mitigate we use an effective brush size of at least 1/4 of the largest
      // side of the area. This reduces the number of centers for small brush sizes
      // and keeps behavior identical for large brushes.
      let area_w = max_x - min_x;
      let area_h = max_y - min_y;
      let area_dim = area_w.max(area_h);
      let override_size = (area_dim / 4.0).max(1.0);
      let effective_size = (brush.size() as f32).max(override_size);
      let radius = effective_size / 2.0;
      // Use smaller stride (one-third radius) for better overlap between dabs and avoid scalloping
      // at area edges. The stride shouldn't be below 1.0 pixel to prevent extremely dense loops.
      let stride = (radius / 3.0).max(1.0);
      let mut centers: Vec<PointF> = Vec::new();

      let mut y = (min_y - radius).floor();
      while y <= (max_y + radius).ceil() {
        let mut x = (min_x - radius).floor();
        while x <= (max_x + radius).ceil() {
          // Include the center if the center itself is inside the coverage
          // or if any of the cardinal points at radius distance is inside (disk intersects).
          // Detect whether the dab circle intersects the coverage by sampling
          // a set of points around the circle's perimeter (cardinal + diagonal).
          let diag = (radius * 0.70710678) as f32; // r / sqrt(2)
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
        let inner_shader = shader_from_fill_with_path(brush.color().clone(), Some(bounds_path));
        let shader: Box<dyn Shader + Send + Sync> =
          Box::new(BrushDabsShader::new(inner_shader, centers, radius, brush.hardness()));
        let compositor = SourceOverCompositor;
        let sample_grid = SampleGrid::from_aa_level(2);
        let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);
        rasterizer.rasterize(self.image);
      }
    }
  }
}
/// Paints with a brush at a specific position using a temporary painter.
/// - `image`: The target image to paint on.
/// - `p_x`: The x-coordinate to paint at.
/// - `p_y`: The y-coordinate to paint at.
/// - `brush`: The brush to use for painting.
pub fn paint_with_brush(image: &mut Image, p_x: impl Into<f64>, p_y: impl Into<f64>, brush: &Brush) {
  let mut painter = Painter::new(image);
  painter.dab_brush(p_x.into() as f32, p_y.into() as f32, brush);
}
/// Strokes a path with a brush to create a continuous line using a temporary painter.
/// - `image`: The target image to paint on.
/// - `path`: The path to stroke.
/// - `brush`: The brush to use for stroking.
pub fn stroke_with_brush(image: &mut Image, path: &Path, brush: &Brush) {
  let mut painter = Painter::new(image);
  painter.stroke_with_brush(path, brush);
}
/// Fills an area with a brush using a temporary painter.
/// - `image`: The target image to paint on.
/// - `area`: The area to fill.
/// - `brush`: The brush to use for filling.
pub fn fill_area_with_brush(image: &mut Image, area: &Area, brush: &Brush) {
  let mut painter = Painter::new(image);
  painter.fill_area_with_brush(area, brush);
}
