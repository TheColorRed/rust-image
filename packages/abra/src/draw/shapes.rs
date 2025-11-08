use crate::{
  color::Color,
  geometry::{
    path::{Path, Rect},
    point::Point,
  },
  image::Image,
};
use rayon::prelude::*;

/// Draws a rectangle.
pub fn rect(image: &mut Image, start: Point, rect: Path, color: Color)
where
  Path: Rect,
{
  let mut pixels = image.rgba();
  let (width, _) = image.dimensions::<i32>();
  let (pos_x, pos_y) = start.dimensions();
  let (pos_x, pos_y) = (pos_x as i32, pos_y as i32);
  let (rect_width, rect_height) = rect.dimensions();
  let (rect_width, rect_height) = (rect_width as i32, rect_height as i32);

  // Draw the rectangle on the image at the given position with the given color.
  pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
    let x = i as i32 % width;
    let y = (i as i32 / width) as i32;
    if x >= pos_x && x < pos_x + rect_width && y >= pos_y && y < pos_y + rect_height {
      let (r, g, b, a) = color.rgba();
      let alpha = a as f32 / 255.0;

      // Extract the existing pixel color
      let existing_pixel = &pixel[..4];
      let existing_r = existing_pixel[0] as f32;
      let existing_g = existing_pixel[1] as f32;
      let existing_b = existing_pixel[2] as f32;
      let existing_a = existing_pixel[3]; // Keep the existing alpha value

      // Blend the colors
      let new_r = (r as f32 * alpha + existing_r * (1.0 - alpha)) as u8;
      let new_g = (g as f32 * alpha + existing_g * (1.0 - alpha)) as u8;
      let new_b = (b as f32 * alpha + existing_b * (1.0 - alpha)) as u8;

      pixel.copy_from_slice(&[new_r, new_g, new_b, existing_a as u8]);
    }
  });

  image.set_rgba(pixels);
}

/// Draws an ellipse.
pub fn ellipse_filled(image: &mut Image, center: Point, size: Path, color: Color)
where
  Path: Rect,
{
  let mut pixels = image.rgba();
  let (width, _) = image.dimensions::<u32>();
  let width = width as i32;
  let (center_x, center_y) = center.dimensions();
  let (center_x, center_y) = (center_x as i32, center_y as i32);
  let (size_width, size_height) = size.dimensions();
  let (size_width, size_height) = (size_width as i32 / 2, size_height as i32 / 2);
  let (r, g, b, a) = color.rgba();

  // Draw the ellipse on the image at the given position with the given color.
  pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
    let x = (i as i32 % width) as i32;
    let y = (i as i32 / width) as i32;
    let dx = (x - center_x) as f32;
    let dy = (y - center_y) as f32;
    let size_width = size_width as f32;
    let size_height = size_height as f32;
    let ellipse_value = (dx * dx) * (size_height * size_height) + (dy * dy) * (size_width * size_width);
    let ellipse_threshold = (size_width * size_width) * (size_height * size_height);

    if ellipse_value < ellipse_threshold {
      let alpha = a as f32 / 255.0;

      // Supersampling for anti-aliasing
      let mut coverage = 0.0;
      let samples = 8;
      let inv_samples = 1.0 / samples as f32;
      let size_height_sq = size_height * size_height;
      let size_width_sq = size_width * size_width;

      for sx in 0..samples {
        let sub_dx_base = dx + (sx as f32 * inv_samples) - 0.5;
        for sy in 0..samples {
          let sub_dy_base = dy + (sy as f32 * inv_samples) - 0.5;
          let sub_ellipse_value = (sub_dx_base * sub_dx_base) * size_height_sq + (sub_dy_base * sub_dy_base) * size_width_sq;
          if sub_ellipse_value < ellipse_threshold {
            coverage += 1.0;
          }
        }
      }
      coverage *= inv_samples * inv_samples;
      let smooth_alpha = alpha * coverage;

      // Extract the existing pixel color
      let existing_pixel = &pixel[..4];
      let existing_r = existing_pixel[0] as f32;
      let existing_g = existing_pixel[1] as f32;
      let existing_b = existing_pixel[2] as f32;

      // Blend the colors
      let new_r = (r as f32 * smooth_alpha + existing_r * (1.0 - smooth_alpha)) as u8;
      let new_g = (g as f32 * smooth_alpha + existing_g * (1.0 - smooth_alpha)) as u8;
      let new_b = (b as f32 * smooth_alpha + existing_b * (1.0 - smooth_alpha)) as u8;
      let existing_a = existing_pixel[3]; // Extract the existing alpha value

      pixel.copy_from_slice(&[new_r, new_g, new_b, existing_a]);
    }
  });

  image.set_rgba(pixels);
}

/// Draws an ellipse outline.
pub fn ellipse_stroke(image: &mut Image, center: Point, size: Path, color: Color, stroke_width: u32)
where
  Path: Rect,
{
  let mut pixels = image.rgba();
  let (width, _) = image.dimensions::<u32>();
  let width = width as i32;
  let (center_x, center_y) = center.dimensions();
  let (center_x, center_y) = (center_x as i32, center_y as i32);
  let (size_width, size_height) = size.dimensions();
  let (size_width, size_height) = (size_width as i32 / 2, size_height as i32 / 2);
  let (r, g, b, a) = color.rgba();

  // Draw the ellipse on the image at the given position with the given color.
  pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
    let x = (i as i32 % width) as i32;
    let y = (i as i32 / width) as i32;
    let dx = (x - center_x) as f32;
    let dy = (y - center_y) as f32;
    let size_width = size_width as f32;
    let size_height = size_height as f32;
    let ellipse_threshold = (size_width * size_width) * (size_height * size_height);

    let alpha = a as f32 / 255.0;

    // Supersampling for anti-aliasing
    let mut coverage = 0.0;
    // Dynamically adjust the number of samples based on the stroke width
    let samples = (stroke_width as f32 * 0.5).max(2.0) as u32; // Increase the minimum value for samples
    let inv_samples = 1.0 / samples as f32;
    let size_height_sq = size_height * size_height;
    let size_width_sq = size_width * size_width;

    for sx in 0..samples {
      let sub_dx_base = dx + (sx as f32 * inv_samples) - 0.5;
      for sy in 0..samples {
        let sub_dy_base = dy + (sy as f32 * inv_samples) - 0.5;
        let sub_ellipse_value = (sub_dx_base * sub_dx_base) * size_height_sq + (sub_dy_base * sub_dy_base) * size_width_sq;
        if sub_ellipse_value < ellipse_threshold {
          coverage += 1.0;
        }
      }
    }
    coverage *= inv_samples * inv_samples;
    let smooth_alpha = alpha * coverage;

    // Extract the existing
    let existing_pixel = &pixel[..4];
    let existing_r = existing_pixel[0] as f32;
    let existing_g = existing_pixel[1] as f32;
    let existing_b = existing_pixel[2] as f32;
    let existing_a = existing_pixel[3] as f32 / 255.0;

    let is_part_of_stroke = is_part_of_stroke(
      x as f32,
      y as f32,
      center_x as f32,
      center_y as f32,
      size_width,
      size_height as f32,
      stroke_width,
    );
    let (new_r, new_g, new_b, new_a) = if is_part_of_stroke {
      // Use the stroke color with anti-aliased value
      let new_r = (r as f32 * (smooth_alpha * 1.5).min(1.0) + existing_r * (1.0 - (smooth_alpha * 1.5).min(1.0))) as u8;
      let new_g = (g as f32 * (smooth_alpha * 1.5).min(1.0) + existing_g * (1.0 - (smooth_alpha * 1.5).min(1.0))) as u8;
      let new_b = (b as f32 * (smooth_alpha * 1.5).min(1.0) + existing_b * (1.0 - (smooth_alpha * 1.5).min(1.0))) as u8;
      (new_r, new_g, new_b, (a as f32 * 255.0) as u8)
    } else {
      // Outside the ellipse and stroke area, keep the existing color
      (existing_r as u8, existing_g as u8, existing_b as u8, (existing_a * 255.0) as u8)
    };

    // Draw the stroke or fill
    pixel.copy_from_slice(&[new_r, new_g, new_b, new_a as u8]);
  });

  image.set_rgba(pixels);
}

/// Draws a circle.
pub fn circle(image: &mut Image, center: Point, radius: u32, color: Color) {
  let size = Rect::new_rect(radius * 2, radius * 2);
  ellipse_filled(image, center, size, color);
}

/// Draws a circle outline.
pub fn circle_stroke(image: &mut Image, center: Point, radius: u32, color: Color, stroke_width: u32) {
  let size = Rect::new_rect(radius * 2, radius * 2);
  ellipse_stroke(image, center, size, color, stroke_width);
}

/// Draws a polygon.
pub fn polygon(image: &mut Image, start: Point, path: Path, color: Color) {
  // - Draw the polygon on the image at the given position with the given color.
  // - The path is a list of points that make up the polygon and are drawn relative to the start point.
  // - The final resulting polygon is drawn at the start point on the image using anti-aliasing.
  let mut pixels = image.rgba();
  let (width, _) = image.dimensions::<u32>();
  let width = width as i32;
  let (start_x, start_y) = start.dimensions();
  let (start_x, start_y) = (start_x as f32, start_y as f32);

  // Define the start point
  let start_point = (start_x, start_y);

  // Adjust the polygon points relative to the start point
  let adjusted_path: Vec<(f32, f32)> = path
    .get_points()
    .iter()
    .map(|&point| (point.x() as f32, point.y() as f32))
    .collect();

  // Draw the polygon on the image at the given position with the given color.
  pixels.par_chunks_mut(4).enumerate().for_each(|(i, pixel)| {
    let x = (i as i32 % width) as f32;
    let y = (i as i32 / width) as f32;
    if point_in_polygon(
      ((x - start_point.0) as i32, (y - start_point.1) as i32),
      &Path::new(adjusted_path.iter().map(|&(px, py)| Point::new(px as i32, py as i32)).collect()),
    ) {
      let (r, g, b, a) = color.rgba();
      let alpha = a as f32 / 255.0;

      // Extract the existing pixel color
      let existing_pixel = &pixel[..4];
      let existing_r = existing_pixel[0] as f32;
      let existing_g = existing_pixel[1] as f32;
      let existing_b = existing_pixel[2] as f32;
      let existing_a = existing_pixel[3]; // Keep the existing alpha value

      // Blend the colors
      let new_r = (r as f32 * alpha + existing_r * (1.0 - alpha)) as u8;
      let new_g = (g as f32 * alpha + existing_g * (1.0 - alpha)) as u8;
      let new_b = (b as f32 * alpha + existing_b * (1.0 - alpha)) as u8;

      pixel.copy_from_slice(&[new_r, new_g, new_b, existing_a as u8]);
    }
  });

  image.set_rgba(pixels);
}

fn point_in_polygon(point: (i32, i32), path: &Path) -> bool {
  let mut inside = false;
  let points = path.get_points();
  let mut j = points.len() - 1;
  let point = (point.0 as f32, point.1 as f32);
  for i in 0..points.len() {
    let (xi, yi) = (points[i].x() as f32, points[i].y() as f32);
    let (xj, yj) = (points[j].x() as f32, points[j].y() as f32);
    if (yi < point.1 && yj >= point.1) || (yj < point.1 && yi >= point.1) {
      if xi + (point.1 - yi) / (yj - yi) * (xj - xi) < point.0 {
        inside = !inside;
      }
    }
    j = i;
  }
  inside
}

fn is_part_of_stroke(x: f32, y: f32, center_x: f32, center_y: f32, radius_x: f32, radius_y: f32, stroke_width: u32) -> bool {
  let dx = x - center_x;
  let dy = y - center_y;
  let stroke = stroke_width as f32 / 2.0;

  // Calculate the squared distances
  let distance_squared = (dx * dx) / (radius_x * radius_x) + (dy * dy) / (radius_y * radius_y);

  // Calculate the outer and inner ellipse thresholds
  let outer_threshold = ((radius_x + stroke) * (radius_y + stroke)) / (radius_x * radius_y);
  let inner_threshold = ((radius_x - stroke) * (radius_y - stroke)) / (radius_x * radius_y);

  // Check if the point is within the stroke area
  distance_squared >= inner_threshold && distance_squared <= outer_threshold
}
