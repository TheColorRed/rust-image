use abra_core::{Area, Fill, Image, Path, PointF};

use crate::shaders::fill_feather_shader::FillFeatherShader;
use crate::{PolygonCoverage, Rasterizer, SampleGrid, SourceOverCompositor, shader_from_fill_with_path};

/// A fill that has been described but not yet drawn. Create one with [`fill`], then either draw it into an existing
/// image with [`FillArea::apply`] or get it as a new image with [`FillArea::to_image`] or `into()`.
#[derive(Clone)]
pub struct FillArea<'a> {
  area: Area,
  fill: Fill<'a>,
  position: Option<PointF>,
}

impl<'a> FillArea<'a> {
  /// Sets where [`FillArea::apply`] draws the top-left corner of the filled area. Defaults to the top-left corner
  /// of the area's bounds, so the area lands where it was drawn.
  pub fn with_position(mut self, p_position: impl Into<PointF>) -> Self {
    self.position = Some(p_position.into());
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

    image
  }

  /// Draws the filled area into an existing image.
  pub fn apply(&self, p_image: &mut Image) {
    let filled = self.to_image();
    let position = self.position.unwrap_or_else(|| {
      let (min_x, min_y, _, _) = self.area.bounds().edges::<f32>();
      PointF::new(min_x, min_y)
    });
    let position: (i32, i32) = position.into();
    p_image.draw_image_at(&filled, position);
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
/// - `p_fill`: The fill style to use.
pub fn fill<'a>(p_area: impl Into<Area>, p_fill: impl Into<Fill<'a>>) -> FillArea<'a> {
  FillArea {
    area: p_area.into(),
    fill: p_fill.into(),
    position: None,
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
  use abra_core::Color;

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
}
