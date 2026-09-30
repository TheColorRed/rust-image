use crate::common::*;
use abra::{
  abra_core::blend::{self, blend_images},
  drawing::prelude::fill,
};
use alakazam::color::Color;

#[napi(object)]
pub struct HueData {
  pub hue: u32,
  pub hex: String,
  pub offset: f64,
}

#[napi]
/// Creates a color picker image based on the active color.
/// @param activeColor The current selected color.
/// @param width The width of the color picker.
/// @param height The height of the color picker.
/// @return The generated color picker image.
pub fn color_picker(active_color: &Color, width: u32, height: u32) -> ImageData {
  // Calculate what hue the active color is
  let h = active_color.hsv().0;
  let hue = Color::from_hsv(h as f64, 1.0, 1.0).into();

  // let active_color: Color = active_color.clone().into();
  let black = Color::black().into();
  let white = Color::white().into();
  let transparent = Color::transparent().into();

  let white_to_hue = Gradient::from_to(white, hue).with_direction(Path::line((0, 0), (width, 0)));
  let trans_to_black = Gradient::from_to(transparent, black).with_direction(Path::line((0, 0), (0, height)));
  let area = Area::rect((0.0, 0.0), (width as f32, height as f32));

  let mut img = Image::new(width, height);
  let img_white_to_hue = fill(&area, &white_to_hue);
  let img_trans_to_black = fill(&area, &trans_to_black);

  img.draw_image_at(&img_white_to_hue, (0, 0));
  blend_images(&mut img, &img_trans_to_black, BlendMode::Normal);

  img.into()
}

#[napi]
/// Gets the color at a specific position in the HSV color space.
/// @param hue The hue value (0-360).
/// @param x The x position (0-1) representing saturation.
/// @param y The y position (0-1) representing value.
/// @return The color at the specified position.
pub fn get_color_at(hue: f64, x: f64, y: f64) -> Color {
  let x = x.clamp(0.0, 1.0);
  let y = y.clamp(0.0, 1.0);
  Color::from_hsv(hue, x, 1.0 - y)
}

#[napi]
/// Gets the cursor position in the color picker based on the given color.
/// @param color The color to get the cursor position for.
/// @param width The width of the color picker.
/// @param height The height of the color picker.
/// @return The (x, y) position of the cursor.
pub fn get_color_picker_cursor_position(color: &Color, width: u32, height: u32) -> (i32, i32) {
  let hsv = color.hsv();
  let x = (hsv.1 * (width as f64)) as i32;
  let y = ((1.0 - hsv.2) * (height as f64)) as i32;
  (x, y)
}

#[napi]
/// Creates a vertical hue gradient image where the gradient start from top and goes to the bottom.
/// @param width The width of the image.
/// @param height The height of the image.
pub fn gradient_hue(width: u32, height: u32) -> ImageData {
  let mut gradient = Gradient::hue();
  let direction = Path::line((0.0, 0.0), (0.0, height as f32));
  gradient = gradient.with_direction(direction);
  fill(&Area::rect((0.0, 0.0), (width as f32, height as f32)), &gradient).into()
}

#[napi]
/// Gets the hue value at a specific position in the hue gradient.
/// @param y The y position in pixels (0-height).
/// @param height The height of the hue gradient.
/// @return The hue data including value (0-360), hex color, and normalized offset.
pub fn get_hue_at(y: f64, height: f64) -> HueData {
  let y = y.clamp(0.0, height);
  let offset = y / height;
  let hue = (offset * 360.0) as u32;
  let color = Color::from_hsv(hue as f64, 1.0, 1.0);
  let hex = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());

  HueData { hue, hex, offset }
}

#[napi]
/// Gets the cursor position in the hue gradient based on the given hue value.
/// @param hue The hue value (0-360).
/// @param height The height of the hue gradient.
/// @return The (x, y) position of the cursor.
pub fn get_hue_cursor_position(hue: f64, height: u32) -> u32 {
  let y = (hue / 360.0) * (height as f64);
  y as u32
}
