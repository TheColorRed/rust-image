use core::{Area, Fill, Image, Path, PointF};

use crate::{PolygonCoverage, Rasterizer, SampleGrid, SourceOverCompositor, shader_from_fill_with_path};
use crate::shaders::fill_feather_shader::FillFeatherShader;

/// Fills the area with the specified fill style.
/// - `p_area`: The area to fill.
/// - `p_fill`: The fill type to use on the area.
pub fn fill(p_area: impl Into<Area>, p_fill: impl Into<Fill>) -> Image {
  let fill = p_fill.into();
  let area = p_area.into();
  let (min_x, min_y, max_x, max_y) = area.bounds::<f32>();
  let width = (max_x - min_x).ceil();
  let height = (max_y - min_y).ceil();

  if width <= 0.0 || height <= 0.0 {
    return Image::new(1, 1);
  }

  let mut image = Image::new(width as u32, height as u32);

  // Flatten the path and translate to image-local coordinates
  let tolerance = 0.5;
  let flattened: Vec<PointF> = area.path.flatten(tolerance).iter().map(|p| PointF::new(p.x - min_x, p.y - min_y)).collect();

  // Build coverage mask
  let coverage = PolygonCoverage::new(flattened.clone());

  // Build shader from fill. If the gradient has no explicit direction, use the
  // area bounding box to create a horizontal gradient path so the gradient
  // is visible across the area.
  let fallback_path = Some(Path::line((min_x, min_y), (max_x, min_y)));
  let mut shader = shader_from_fill_with_path(fill.clone(), fallback_path);
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

  image
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::{Area, Color};

  #[test]
  fn fill_with_feather_sets_alpha_near_edge() {
    // 20x20 image, rectangle area with feather 4.
    let area = Area::rect((2.0, 2.0), (16.0, 16.0)).with_feather(4);
    let img = fill(area, Color::from_rgba(0, 0, 0, 255));
    // Check center pixel alpha (should be fully opaque)
    let (w, h) = img.dimensions::<u32>();
    let cx = w / 2;
    let cy = h / 2;
    assert_eq!(img.get_pixel(cx, cy).unwrap().3, 255);
    // Check a pixel near the edge (should be less than 255 if inside feather)
    let near_edge = img.get_pixel(3, 3).unwrap().3;
    assert!(near_edge < 255);
  }
}
