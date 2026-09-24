use abra_core::{Color, Fill, Image, Path, Point};

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;

use drawing::Brush;
use drawing::Painter;

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
pub struct Stroke<'a> {
  /// The color of the outline in RGBA format.
  pub fill: Fill<'a>,
  /// The blend mode used to combine the outline with the layer.
  pub opacity: f32,
  /// The thickness of the outline.
  pub size: u32,
  /// The position of the outline relative to the shape.
  pub position: OutlinePosition,
}

impl<'a> Stroke<'a> {
  /// Creates a new StrokeOptions with default settings.
  /// Default values:
  /// - size: 3.0 pixels
  /// - color: black with 100% opacity (0, 0, 0, 255)
  pub fn new() -> Self {
    Stroke {
      fill: Fill::Solid(Cow::Owned(Color::black())),
      opacity: 1.0,
      size: 3,
      position: OutlinePosition::Inside,
    }
  }

  /// Sets the size of the outline.
  pub fn with_size(mut self, p_size: u32) -> Self {
    self.size = p_size;
    self
  }

  /// Sets the fill of the outline.
  pub fn with_fill(mut self, p_fill: Fill<'a>) -> Self {
    self.fill = p_fill;
    self
  }

  /// Sets the opacity of the outline.
  pub fn with_opacity(mut self, p_opacity: f32) -> Self {
    self.opacity = p_opacity;
    self
  }

  /// Sets the position of the stroke relative to the path: Inside, Outside or Center.
  pub fn with_position(mut self, p_position: OutlinePosition) -> Self {
    self.position = p_position;
    self
  }
}

/// Applies a stroke effect to an image by drawing an outline around its edges.
pub(crate) fn apply_stroke(p_image: Arc<Image>, p_options: &Stroke) -> Arc<Image> {
  let _duration = Instant::now();
  let original_image = p_image.as_ref();
  let (width, height) = original_image.dimensions::<u32>();

  let mut composite = Image::new(width, height);
  composite.copy_channel_data(original_image);

  let max_x = (width.saturating_sub(1)) as i32;
  let max_y = (height.saturating_sub(1)) as i32;

  // We will build our path at the original image border coordinates.
  let relative_path = Point::array(vec![(0, 0), (max_x, 0), (max_x, max_y), (0, max_y), (0, 0)]);

  // Create a path from the calculated points.
  let mut path = Path::new();
  if !relative_path.is_empty() {
    path.move_to((relative_path[0].x() as f32, relative_path[0].y() as f32));
    for point in relative_path.iter().skip(1) {
      path.line_to((point.x() as f32, point.y() as f32));
    }
  }

  // Respect configured opacity by adjusting fill alpha and materialize a Solid fill.
  let solid_color = match &p_options.fill {
    Fill::Solid(cow) => {
      // Ensure owned Color so we can mutate alpha regardless of borrow state.
      let mut col = cow.clone().into_owned();
      let factor = p_options.opacity.clamp(0.0, 1.0);
      col.a = ((col.a as f32) * factor).round().clamp(0.0, 255.0) as u8;
      col
    }
    _ => {
      let mut col = Color::black();
      let factor = p_options.opacity.clamp(0.0, 1.0);
      col.a = ((col.a as f32) * factor).round().clamp(0.0, 255.0) as u8;
      col
    }
  };

  let fill = Fill::Solid(Cow::Owned(solid_color));

  // Use the Painter to render the stroke using a brush. Build a brush based on options
  // with a hardness of 1.0 and paint the path directly into the composite image.
  let brush = Brush::new().with_size(p_options.size).with_color(fill).with_hardness(1.0);

  let mut painter = Painter::new(&mut composite);
  // Pass ownership of the mask to the shader (clone the area)
  painter.stroke_with_brush(&path, &brush);

  // DebugEffects::Stroke(p_options.clone(), duration.elapsed()).log();

  Arc::new(composite)
}
