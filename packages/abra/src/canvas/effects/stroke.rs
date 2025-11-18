use crate::color::{Color, Fill};
use crate::{
  Image,
  brush::Brush,
  draw::painter::Painter,
  geometry::{Path, Point},
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

  // Build a path that goes to each corner of the image so the stroke is drawn on
  // the image border. Apply an offset based on `OutlinePosition` so clients can
  // request an inside/center/outside-follow behavior.
  let max_x = (width.saturating_sub(1)) as i32;
  let max_y = (height.saturating_sub(1)) as i32;

  let offset = match options.position {
    OutlinePosition::Inside => (options.size as i32) / 2,
    OutlinePosition::Outside => -((options.size as i32) / 2),
    OutlinePosition::Center => 0,
  };

  let relative_path = vec![
    Point::new(0 + offset, 0 + offset),
    Point::new(max_x + offset, 0 + offset),
    Point::new(max_x + offset, max_y + offset),
    Point::new(0 + offset, max_y + offset),
    Point::new(0 + offset, 0 + offset),
  ];

  // Cap type is intentionally ignored by the painter stroke method for now.

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

  // Use the Painter to render the stroke using a brush. Build a brush based on options
  // with a hardness of 1.0 and paint the path directly into the composite image.
  let brush = Brush::new()
    .with_size(options.size)
    .with_color(fill.clone())
    .with_hardness(1.0);

  // Build the final path from the corner points (no translation so no padding)
  let mut translated_path = Path::new();
  if !relative_path.is_empty() {
    translated_path.with_move_to((relative_path[0].x() as f32, relative_path[0].y() as f32));
    for point in relative_path.iter().skip(1) {
      translated_path.with_line_to((point.x() as f32, point.y() as f32));
    }
  }

  let mut painter = Painter::new(&mut composite);
  painter.stroke_with_brush(&translated_path, &brush);

  // Restore the original alpha channel to prevent dark edges at the boundary
  // This ensures the stroke only blends within the original layer's boundaries
  let original_pixels = original_image.rgba();
  let mut composite_pixels = composite.rgba();

  // Preserve any added stroke alpha while avoiding lowering alpha where the
  // original image has a higher alpha value. This allows strokes to show on
  // the image border even if the original image has transparent corners.
  for i in (3..composite_pixels.len()).step_by(4) {
    let orig_a = original_pixels[i];
    let comp_a = composite_pixels[i];
    composite_pixels[i] = comp_a.max(orig_a);
  }

  composite.set_rgba(composite_pixels);

  DebugEffects::Stroke(options.clone(), duration.elapsed()).log();

  Arc::new(composite)
}
