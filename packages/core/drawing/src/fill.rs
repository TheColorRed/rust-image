use core::{Area, Fill, Image, Path, PointF};

use crate::{PolygonCoverage, Rasterizer, SampleGrid, SourceOverCompositor, shader_from_fill_with_path};

/// Fills the area with the specified fill style.
/// - `p_area`: The area to fill.
/// - `p_fill`: The fill type to use on the area.
pub fn fill(p_area: impl Into<Area>, p_fill: impl Into<Fill>) -> Image {
  let fill = p_fill.into();
  let area = p_area.into();
  let (min_x, min_y, max_x, max_y) = area.bounds();
  let width = (max_x - min_x).ceil();
  let height = (max_y - min_y).ceil();

  if width <= 0.0 || height <= 0.0 {
    return Image::new(1, 1);
  }

  let mut image = Image::new(width as u32, height as u32);

  // Flatten the path and translate to image-local coordinates
  let tolerance = 0.5;
  let flattened: Vec<PointF> = area
    .path
    .flatten(tolerance)
    .iter()
    .map(|p| PointF::new(p.x - min_x, p.y - min_y))
    .collect();

  // Build coverage mask
  let coverage = PolygonCoverage::new(flattened);

  // Build shader from fill. If the gradient has no explicit direction, use the
  // area bounding box to create a horizontal gradient path so the gradient
  // is visible across the area.
  let fallback_path = Some(Path::line((min_x, min_y), (max_x, min_y)));
  let shader = shader_from_fill_with_path(fill.clone(), fallback_path);

  // Use source-over compositing
  let compositor = SourceOverCompositor;

  // Use anti-aliasing level from image
  let sample_grid = SampleGrid::from_aa_level(image.anti_aliasing_level);

  // Rasterize
  let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, sample_grid);
  rasterizer.rasterize(&mut image);

  image
}
