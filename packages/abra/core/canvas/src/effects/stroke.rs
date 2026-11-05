use abra_core::{Color, Fill, Image, Path, PointF};

use abra_core::image::recipe::{gpu_ready, lay_over};
use abra_core::BlendMode;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use drawing::Brush;
use drawing::Painter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
      fill: Fill::Solid(Color::black()),
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

/// The outlines drawn lately, as a transparent picture with only the outline in it, so a stroke that does not change is drawn once and not
/// each time the layer is composed. Keyed by the size of the picture, the thickness and the color.
static OUTLINES: Mutex<Vec<((u32, u32, u32, [u8; 4]), Image)>> = Mutex::new(Vec::new());

/// How many outlines are kept.
const OUTLINES_KEPT: usize = 8;

/// The color the outline is drawn in, with the opacity folded into its alpha (a fill that is not a solid color is black).
fn outline_color(p_options: &Stroke) -> Color {
  let mut color = match &p_options.fill {
    Fill::Solid(cow) => *cow,
    _ => Color::black(),
  };
  color.a = ((color.a as f32) * p_options.opacity.clamp(0.0, 1.0)).round().clamp(0.0, 255.0) as u8;
  color
}

/// Draws the outline of a picture of `p_width` x `p_height` with `painter` on `p_target`.
fn draw_outline(p_target: &mut Image, p_options: &Stroke) {
  let (width, height) = p_target.dimensions::<u32>();
  let (max_x, max_y) = (width.saturating_sub(1) as i32, height.saturating_sub(1) as i32);
  let corners: Vec<PointF> = [(0, 0), (max_x, 0), (max_x, max_y), (0, max_y), (0, 0)].into_iter().map(PointF::from).collect();
  let mut path = Path::new();
  path.move_to((corners[0].x, corners[0].y));
  for point in corners.iter().skip(1) {
    path.line_to((point.x, point.y));
  }
  let brush = Brush::new().with_size(p_options.size).with_color(Fill::Solid(outline_color(p_options))).with_hardness(1.0);
  Painter::new(p_target).stroke_with_brush(&path, &brush);
}

/// Applies a stroke effect to an image by drawing an outline around its edges.
pub(crate) fn apply_stroke(p_image: Arc<Image>, p_options: &Stroke) -> Arc<Image> {
  let _duration = Instant::now();
  let original_image = p_image.as_ref();
  let (width, height) = original_image.dimensions::<u32>();

  // With a GPU the outline is a transparent picture laid over the layer as one more step of its chain, and the layer's pixels are not
  // copied or read.
  if gpu_ready() {
    let color = outline_color(p_options);
    let key = (width, height, p_options.size, [color.r, color.g, color.b, color.a]);
    let outline = {
      let mut kept = OUTLINES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      match kept.iter().position(|(entry, _)| *entry == key) {
        Some(at) => kept[at].1.clone(),
        None => {
          let mut outline = Image::new(width, height);
          draw_outline(&mut outline, p_options);
          if kept.len() >= OUTLINES_KEPT {
            kept.remove(0);
          }
          kept.push((key, outline.clone()));
          outline
        }
      }
    };
    let mut composite = original_image.clone();
    lay_over(&mut composite, &outline, (0, 0), BlendMode::Normal, 1.0);
    return Arc::new(composite);
  }

  let mut composite = Image::new(width, height);
  composite.copy_channel_data(original_image);

  let max_x = (width.saturating_sub(1)) as i32;
  let max_y = (height.saturating_sub(1)) as i32;

  // We will build our path at the original image border coordinates.
  let relative_path: Vec<PointF> =
    [(0, 0), (max_x, 0), (max_x, max_y), (0, max_y), (0, 0)].into_iter().map(PointF::from).collect();

  // Create a path from the calculated points.
  let mut path = Path::new();
  if !relative_path.is_empty() {
    path.move_to((relative_path[0].x, relative_path[0].y));
    for point in relative_path.iter().skip(1) {
      path.line_to((point.x, point.y));
    }
  }

  // Respect configured opacity by adjusting fill alpha and materialize a Solid fill.
  let solid_color = match &p_options.fill {
    Fill::Solid(cow) => {
      // Ensure owned Color so we can mutate alpha regardless of borrow state.
      let mut col = *cow;
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

  let fill = Fill::Solid(solid_color);

  // Use the Painter to render the stroke using a brush. Build a brush based on options
  // with a hardness of 1.0 and paint the path directly into the composite image.
  let brush = Brush::new().with_size(p_options.size).with_color(fill).with_hardness(1.0);

  let mut painter = Painter::new(&mut composite);
  // Pass ownership of the mask to the shader (clone the area)
  painter.stroke_with_brush(&path, &brush);

  // DebugEffects::Stroke(p_options.clone(), duration.elapsed()).log();

  Arc::new(composite)
}
