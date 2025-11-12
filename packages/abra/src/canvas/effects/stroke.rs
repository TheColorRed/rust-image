use std::time::Instant;

use crate::{
  Image, Layer,
  canvas::{StrokeOptions, effects::options_stroke::OutlinePosition},
  draw::line,
  geometry::{path::Path, point::Point},
  utils::debug::DebugEffects,
};

/// Applies a stroke effect to a layer by drawing an outline around its edges.
pub fn stroke(layer: Layer, options: StrokeOptions) {
  let duration = Instant::now();
  let mut layer_inner = layer.borrow_mut();
  let original_image = layer_inner.image().clone();

  // Get layer dimensions
  let (width, height) = original_image.dimensions::<u32>();

  let mut composite = Image::new(width, height);
  composite.set_rgba(original_image.rgba());

  let start = match options.position {
    OutlinePosition::Inside => Point::new(options.size as i32 / 2, options.size as i32 / 2),
    OutlinePosition::Outside => Point::new(-(options.size as i32 / 2), -(options.size as i32 / 2)),
    OutlinePosition::Center => Point::new(0, 0),
  };

  let stroke_max_width = match options.position {
    // Image width - stroke size for inside
    OutlinePosition::Inside => width - options.size,
    OutlinePosition::Outside => width + options.size,
    OutlinePosition::Center => 0,
  };

  let stroke_max_height = match options.position {
    // Image height - stroke size for inside
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
    OutlinePosition::Inside => line::LineCap::Square,
    OutlinePosition::Outside => line::LineCap::Round,
    OutlinePosition::Center => line::LineCap::Round,
  };

  let fill = options.fill.clone();
  line::line(&mut composite, start, Path::new(relative_path), fill, options.size, Some(cap));

  layer_inner.image_mut().set_new_pixels(composite.rgba(), width, height);

  DebugEffects::Stroke(options, duration.elapsed()).log();
}
