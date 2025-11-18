use crate::{
  Image,
  brush::Brush,
  draw::core::{
    BrushShader, PolygonCoverage, Rasterizer, SampleGrid, Shader, SourceOverCompositor, StrokeBrushShader,
    shader_from_fill,
  },
  geometry::{Area, LineCap, LineJoin, Path, PointF},
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
    let inner_shader = shader_from_fill(fill);
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
    let inner_shader = shader_from_fill(brush.color());
    // Path stroke shading falloff radius is (width / 2)
    let max_distance = width / 2.0;
    let shader: Box<dyn Shader + Send + Sync> =
      Box::new(StrokeBrushShader::new(inner_shader, path.clone(), max_distance, brush.hardness()));

    let compositor = SourceOverCompositor;
    let sample_grid = SampleGrid::from_aa_level(2);
    let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);

    rasterizer.rasterize(self.image);
  }
}
