use crate::color::{Color, Fill};
use crate::{
  Image,
  geometry::{Area, LineCap, LineJoin, Path, Point},
  utils::debug::DebugEffects,
};

use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug)]
/// Position of the outline relative to the shape.
pub enum OutlinePosition {
  /// Outline is drawn inside the shape.
  Inside,
  /// Outline is drawn outside the shape.
  Outside,
  /// Outline is centered on the shape's edge. Half inside, half outside.
  Center,
}

#[derive(Clone, Debug)]
/// Options for configuring a stroke effect.
pub struct Stroke {
  /// The color of the outline in RGBA format.
  pub fill: Fill,
  /// The blend mode used to combine the outline with the layer.
  pub opacity: f32,
  /// The thickness of the outline.
  pub size: u32,
  /// The position of the outline relative to the shape.
  pub position: OutlinePosition,
}

impl Stroke {
  /// Creates a new StrokeOptions with default settings.
  /// Default values:
  /// - size: 3.0 pixels
  /// - color: black with 100% opacity (0, 0, 0, 255)
  pub fn new() -> Self {
    Stroke {
      fill: Fill::Solid(Color::black()),
      opacity: 1.0,
      size: 3,
      position: OutlinePosition::Inside,
    }
  }

  /// Sets the size of the outline.
  pub fn with_size(mut self, size: u32) -> Self {
    self.size = size;
    self
  }

  /// Sets the fill of the outline.
  pub fn with_fill(mut self, fill: Fill) -> Self {
    self.fill = fill;
    self
  }

  /// Sets the opacity of the outline.
  pub fn with_opacity(mut self, opacity: f32) -> Self {
    self.opacity = opacity;
    self
  }
}

/// Applies a stroke effect to an image by drawing an outline around its edges.
pub(crate) fn apply_stroke(image: Arc<Image>, options: &Stroke) -> Arc<Image> {
  let duration = Instant::now();
  let original_image = image.as_ref();
  let (width, height) = original_image.dimensions::<u32>();

  let mut composite = Image::new(width, height);
  composite.set_rgba(original_image.rgba());

  let start = match options.position {
    OutlinePosition::Inside => Point::new(options.size as i32 / 2, options.size as i32 / 2),
    OutlinePosition::Outside => Point::new(-(options.size as i32 / 2), -(options.size as i32 / 2)),
    OutlinePosition::Center => Point::new(0, 0),
  };

  let stroke_max_width = match options.position {
    OutlinePosition::Inside => width - options.size,
    OutlinePosition::Outside => width + options.size,
    OutlinePosition::Center => 0,
  };

  let stroke_max_height = match options.position {
    OutlinePosition::Inside => height - options.size,
    OutlinePosition::Outside => height + options.size,
    OutlinePosition::Center => 0,
  };

  let relative_path = vec![
    Point::new(0, 0),
    Point::new((stroke_max_width) as i32, 0),
    Point::new((stroke_max_width) as i32, (stroke_max_height) as i32),
    Point::new(0, (stroke_max_height) as i32),
    Point::new(0, 0),
  ];

  let cap = match options.position {
    OutlinePosition::Inside => LineCap::Square,
    OutlinePosition::Outside => LineCap::Round,
    OutlinePosition::Center => LineCap::Round,
  };

  // Respect configured opacity by adjusting fill alpha.
  let fill = match options.fill.clone() {
    Fill::Solid(mut c) => {
      let factor = options.opacity.clamp(0.0, 1.0);
      let new_a = ((c.a as f32) * factor).round().clamp(0.0, 255.0) as u8;
      c.a = new_a;
      Fill::Solid(c)
    }
    other => other,
  };

  // Create path from points using the new API
  let mut path = Path::new();
  if !relative_path.is_empty() {
    path.with_move_to((relative_path[0].x() as f32, relative_path[0].y() as f32));
    for point in relative_path.iter().skip(1) {
      path.with_line_to((point.x() as f32, point.y() as f32));
    }
  }

  // Use the new unified stroke API: stroke path to create outline area, then fill
  let stroke_outline = path.stroke(options.size as f32, LineJoin::Miter, cap);
  let stroke_area: Area = stroke_outline.into();
  let stroke_image = stroke_area.fill(fill.clone());

  // Composite the stroke onto the composite image at the start position
  let stroke_pixels = stroke_image.rgba();
  let (stroke_width, stroke_height) = stroke_image.dimensions::<u32>();
  let mut composite_pixels = composite.rgba();

  for y in 0..stroke_height {
    for x in 0..stroke_width {
      let comp_x = (start.x() + x as i32) as u32;
      let comp_y = (start.y() + y as i32) as u32;

      if comp_x < width && comp_y < height {
        let stroke_idx = ((y * stroke_width + x) as usize) * 4;
        let comp_idx = ((comp_y * width + comp_x) as usize) * 4;

        let src_a = stroke_pixels[stroke_idx + 3] as f32 / 255.0;
        if src_a > 0.0 {
          let dst_a = composite_pixels[comp_idx + 3] as f32 / 255.0;
          let out_a = src_a + dst_a * (1.0 - src_a);

          if out_a > 0.0 {
            for c in 0..3 {
              let src_c = stroke_pixels[stroke_idx + c] as f32;
              let dst_c = composite_pixels[comp_idx + c] as f32;
              let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
              composite_pixels[comp_idx + c] = out_c.round().clamp(0.0, 255.0) as u8;
            }
            composite_pixels[comp_idx + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
          }
        }
      }
    }
  }

  composite.set_rgba(composite_pixels);

  // Restore the original alpha channel to prevent dark edges at the boundary
  // This ensures the stroke only blends within the original layer's boundaries
  let original_pixels = original_image.rgba();
  let mut composite_pixels = composite.rgba();

  for i in (3..composite_pixels.len()).step_by(4) {
    composite_pixels[i] = original_pixels[i];
  }

  composite.set_rgba(composite_pixels);

  DebugEffects::Stroke(options.clone(), duration.elapsed()).log();

  Arc::new(composite)
}
