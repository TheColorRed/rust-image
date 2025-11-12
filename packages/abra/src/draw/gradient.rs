use std::time::Instant;

use crate::color::gradient::Gradient;
use crate::geometry::path::Path;
use crate::image::Image;
use crate::utils::debug::DebugDrawing;
use rayon::prelude::*;

/// Creates a radial gradient starting at x1, y1 and ending at x2, y2.
/// The gradient starts with the color `start` and ends with the color `end`.
/// The gradient is applied to the entire image not just the line.
/// **Note:** the middle is the center of the first color in the list.
pub fn radial_gradient(image: &mut Image, radius: u32, gradient: Gradient) {
  let mut pixels = image.empty_pixel_vec();
  let (width, _) = image.dimensions::<u32>();

  pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
    let x_pixel = i as u32 % width;
    let y_pixel = i as u32 / width;
    let dx = x_pixel as f32 - radius as f32;
    let dy = y_pixel as f32 - radius as f32;
    let distance = (dx * dx + dy * dy).sqrt();
    let time = (distance / radius as f32).min(1.0).max(0.0);

    let (r, g, b, a) = gradient.get_color(time);

    pixel[0] = r;
    pixel[1] = g;
    pixel[2] = b;
    pixel[3] = a;
  });

  image.set_rgba(pixels);
}

/// Creates a linear gradient. The gradient is applied to the entire image.
/// The gradient starts with the first color in the list and ends with the last color in the list.
/// The gradient is applied to the entire image not just the line.
pub fn linear_gradient(image: &mut Image, path: Path, stops: Gradient) {
  let duration = Instant::now();
  image.mut_pixels_with_position(|x, y, mut pixel| {
    let mut r_sum = 0.0;
    let mut g_sum = 0.0;
    let mut b_sum = 0.0;
    let mut a_sum = 0.0;
    let samples = 4; // 2x2 supersampling

    for sx in 0..2 {
      for sy in 0..2 {
        let sub_x = x as f32 + (sx as f32 + 0.5) / 2.0;
        let sub_y = y as f32 + (sy as f32 + 0.5) / 2.0;
        let closest_t = path.closest_time(sub_x, sub_y);
        let (r, g, b, a) = stops.get_color(closest_t);

        r_sum += r as f32;
        g_sum += g as f32;
        b_sum += b as f32;
        a_sum += a as f32;
      }
    }

    pixel[0] = (r_sum / samples as f32) as u8;
    pixel[1] = (g_sum / samples as f32) as u8;
    pixel[2] = (b_sum / samples as f32) as u8;
    pixel[3] = (a_sum / samples as f32) as u8;
  });

  DebugDrawing::Gradient(stops, path, duration.elapsed()).log();
}
