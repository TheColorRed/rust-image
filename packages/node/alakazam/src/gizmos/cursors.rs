use abra::{drawing::prelude::*, prelude::*};

use crate::common::*;

#[napi(namespace = "AlakazamGizmos")]
pub fn circle_cursor(size: u32) -> ImageData {
  let mut cursor = Image::new(size + 4, size + 4);
  let size = size as f32;

  // Draw a black path
  let black_path = Area::ellipse(((size / 2.0) + 2.0, (size / 2.0) + 2.0), (size, size));
  // Draw a white 1px smaller path
  let white_path = Area::ellipse(((size / 2.0) + 2.0, (size / 2.0) + 2.0), ((size) - 2.0, (size) - 2.0));

  let black_color = Color::black();
  let white_color = Color::white();
  let brush_black = Brush::new().with_size(2).with_hardness(1.0).with_color(&black_color);
  let brush_white = Brush::new().with_size(2).with_hardness(1.0).with_color(&white_color);

  draw_area_stroke(&mut cursor, &black_path, &brush_black);
  draw_area_stroke(&mut cursor, &white_path, &brush_white);

  cursor.into()
}

#[napi(namespace = "AlakazamGizmos")]
pub fn arrow_cursor(size: u32, direction: String, color: String) -> ImageData {
  let mut cursor = Image::new(size, size);
  let color = abra::abra_core::Color::from_hex_string(&color);
  let brush_black = Brush::new().with_size(2).with_hardness(1.0).with_color(&color);

  let mut path = Area::new();
  match direction.as_str() {
    "up" => {
      // Pentagon "house" pointing up (roof apex at top center)
      let roof_bottom = size as f32 * 0.55;
      path.move_to((size as f32 / 2.0, 2.0));
      path.line_to((size as f32 - 2.0, roof_bottom));
      path.line_to((size as f32 - 2.0, size as f32 - 2.0));
      path.line_to((2.0, size as f32 - 2.0));
      path.line_to((2.0, roof_bottom));
    }
    "down" => {
      // Pentagon "house" pointing down (apex at bottom center)
      let roof_top = size as f32 * 0.45;
      path.move_to((size as f32 / 2.0, size as f32 - 2.0));
      path.line_to((2.0, roof_top));
      path.line_to((2.0, 2.0));
      path.line_to((size as f32 - 2.0, 2.0));
      path.line_to((size as f32 - 2.0, roof_top));
    }
    "left" => {
      // Pentagon "house" pointing left (apex at left center)
      let roof_right = size as f32 * 0.55;
      path.move_to((2.0, size as f32 / 2.0));
      path.line_to((roof_right, 2.0));
      path.line_to((size as f32 - 2.0, 2.0));
      path.line_to((size as f32 - 2.0, size as f32 - 2.0));
      path.line_to((roof_right, size as f32 - 2.0));
    }
    "right" => {
      // Pentagon "house" pointing right (apex at right center)
      let roof_left = size as f32 * 0.45;
      path.move_to((size as f32 - 2.0, size as f32 / 2.0));
      path.line_to((roof_left, size as f32 - 2.0));
      path.line_to((2.0, size as f32 - 2.0));
      path.line_to((2.0, 2.0));
      path.line_to((roof_left, 2.0));
    }
    _ => {}
  }

  draw_area_fill(&mut cursor, &path, &brush_black);
  cursor.into()
}
