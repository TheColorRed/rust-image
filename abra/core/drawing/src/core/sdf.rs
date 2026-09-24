use abra_core::{Image, PointF};

/// Approximate signed distance for an ellipse centered at (cx,cy) with radii rx, ry.
/// Uses a normalized distance approach and scales by min(rx, ry).
pub fn sdf_ellipse(p_cx: f32, p_cy: f32, p_rx: f32, p_ry: f32, p_x: f32, p_y: f32) -> f32 {
  // Avoid division by zero
  let p_rx = p_rx.max(1e-6);
  let p_ry = p_ry.max(1e-6);
  let dx = (p_x - p_cx) / p_rx;
  let dy = (p_y - p_cy) / p_ry;
  let k = (dx * dx + dy * dy).sqrt();
  // k - 1.0 is the normalized distance; scale by an average radius for approximate pixel units
  let scale = p_rx.min(p_ry);
  (k - 1.0) * scale
}

/// Heuristic: detect if a flattened polyline represents an ellipse/circle.
fn detect_ellipse_from_points(p_points: &[PointF], p_tol: f32) -> Option<(f32, f32, f32, f32)> {
  if p_points.is_empty() {
    return None;
  }
  // Use bounds to get center and radii estimate
  let mut min_x = p_points[0].x;
  let mut min_y = p_points[0].y;
  let mut max_x = p_points[0].x;
  let mut max_y = p_points[0].y;
  for p in p_points.iter() {
    min_x = min_x.min(p.x);
    min_y = min_y.min(p.y);
    max_x = max_x.max(p.x);
    max_y = max_y.max(p.y);
  }
  let rx = (max_x - min_x) / 2.0;
  let ry = (max_y - min_y) / 2.0;
  if rx <= 0.0 || ry <= 0.0 {
    return None;
  }
  let cx = (min_x + max_x) / 2.0;
  let cy = (min_y + max_y) / 2.0;
  // Check each point approximately lies on ellipse
  for p in p_points.iter() {
    let dx = (p.x - cx) / rx;
    let dy = (p.y - cy) / ry;
    let r = (dx * dx + dy * dy).sqrt();
    if (r - 1.0).abs() > p_tol {
      return None;
    }
  }
  Some((cx, cy, rx, ry))
}

/// Draw a filled ellipse using SDF evaluation and simple AA (analytic fast-path for Area fills).
pub fn draw_ellipse_fill(p_img: &mut Image, p_cx: f32, p_cy: f32, p_rx: f32, p_ry: f32, p_brush: &crate::Brush) {
  // Only Solid fill supported
  let (color, opacity) = match p_brush.color() {
    abra_core::Fill::Solid(c) => (c, p_brush.opacity()),
    _ => return,
  };
  let alpha_base = (color.a as f32 / 255.0) * opacity;
  let (min_x, min_y, max_x, max_y) = (
    (p_cx - p_rx).floor().max(0.0) as i32,
    (p_cy - p_ry).floor().max(0.0) as i32,
    (p_cx + p_rx).ceil() as i32,
    (p_cy + p_ry).ceil() as i32,
  );
  let img_w = p_img.dimensions::<u32>().0 as i32;
  let img_h = p_img.dimensions::<u32>().1 as i32;
  let min_x_u = (min_x.max(0)) as u32;
  let max_x_u = (max_x.min(img_w - 1)) as u32;
  let min_y_u = (min_y.max(0)) as u32;
  let max_y_u = (max_y.min(img_h - 1)) as u32;
  let arr = p_img.colors();
  let avg_r = color.r as f32;
  let avg_g = color.g as f32;
  let avg_b = color.b as f32;

  for yy in min_y_u..=max_y_u {
    let y = yy as f32 + 0.5;
    for xx in min_x_u..=max_x_u {
      let x = xx as f32 + 0.5;
      let dist = sdf_ellipse(p_cx, p_cy, p_rx, p_ry, x, y);
      // coverage: inside -> 1, outside ~ linear AA over 1px
      let coverage = (1.0 - dist.clamp(0.0, 1.0)).clamp(0.0, 1.0);
      if coverage <= 0.0 {
        continue;
      }
      let alpha = alpha_base * coverage;
      let img_w_u = img_w as u32;
      let idx = ((yy * img_w_u + xx) as usize) * 4;
      let dst_r = arr[idx] as f32;
      let dst_g = arr[idx + 1] as f32;
      let dst_b = arr[idx + 2] as f32;
      let dst_a = arr[idx + 3] as f32 / 255.0;
      let out_a = alpha + dst_a * (1.0 - alpha);
      let out_r = (avg_r * alpha + dst_r * (1.0 - alpha)).round();
      let out_g = (avg_g * alpha + dst_g * (1.0 - alpha)).round();
      let out_b = (avg_b * alpha + dst_b * (1.0 - alpha)).round();
      arr[idx] = out_r as u8;
      arr[idx + 1] = out_g as u8;
      arr[idx + 2] = out_b as u8;
      arr[idx + 3] = (out_a * 255.0).round() as u8;
    }
  }
}

/// Signed distance to a closed polygon. `p_points` should form a closed ring in order.
/// Returns negative if point is inside (winding rule) and positive if outside.
pub fn sdf_polygon(p_points: &[PointF], p_x: f32, p_y: f32) -> f32 {
  if p_points.is_empty() {
    return f32::MAX;
  }

  // Minimum distance to any segment
  let mut min_dist = f32::MAX;
  for i in 0..p_points.len() {
    let a = p_points[i];
    let b = p_points[(i + 1) % p_points.len()];
    // Project point onto segment
    let pax = p_x - a.x;
    let pay = p_y - a.y;
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let ab_len2 = abx * abx + aby * aby;
    let t = if ab_len2 == 0.0 { 0.0 } else { (pax * abx + pay * aby) / ab_len2 };
    let t_clamped = t.clamp(0.0, 1.0);
    let proj_x = a.x + abx * t_clamped;
    let proj_y = a.y + aby * t_clamped;
    let dx = p_x - proj_x;
    let dy = p_y - proj_y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < min_dist {
      min_dist = dist;
    }
  }

  // Point-in-polygon test (ray casting)
  let mut inside = false;
  for i in 0..p_points.len() {
    let a = p_points[i];
    let b = p_points[(i + 1) % p_points.len()];
    let intersects = ((a.y > p_y) != (b.y > p_y)) && (p_x < (b.x - a.x) * (p_y - a.y) / (b.y - a.y) + a.x);
    if intersects {
      inside = !inside;
    }
  }

  if inside { -min_dist } else { min_dist }
}

/// Draw a stroked ellipse into `p_img` using SDF evaluation. The stroke is centered on the ellipse
/// outline with total width `stroke_w`. `hardness` (0.0..1.0) acts as a multiplier for coverage falloff.
pub fn draw_ellipse_stroke(p_img: &mut Image, p_cx: f32, p_cy: f32, p_rx: f32, p_ry: f32, p_brush: &crate::Brush) {
  // Extract stroke width, color and hardness from brush. Only Solid fills are supported.
  let stroke_w = p_brush.size() as f32;
  let half = stroke_w / 2.0;
  let (color, hardness, opacity) = match p_brush.color() {
    abra_core::Fill::Solid(c) => (c, p_brush.hardness(), p_brush.opacity()),
    _ => {
      // Unsupported brush fill; fallback to no-op
      return;
    }
  };
  let alpha_base = (color.a as f32 / 255.0) * opacity;
  // Bounding box expanded by half width + 1 pixel for AA
  let min_x = ((p_cx - p_rx - half - 1.0).floor() as i32).max(0) as u32;
  let max_x = ((p_cx + p_rx + half + 1.0).ceil() as i32).min(p_img.dimensions::<u32>().0 as i32 - 1) as u32;
  let min_y = ((p_cy - p_ry - half - 1.0).floor() as i32).max(0) as u32;
  let max_y = ((p_cy + p_ry + half + 1.0).ceil() as i32).min(p_img.dimensions::<u32>().1 as i32 - 1) as u32;

  let (img_w, _img_h) = p_img.dimensions::<u32>();
  let arr = p_img.colors();
  let avg_r = color.r as f32;
  let avg_g = color.g as f32;
  let avg_b = color.b as f32;

  for yy in min_y..=max_y {
    let y = yy as f32 + 0.5;
    for xx in min_x..=max_x {
      let x = xx as f32 + 0.5;
      let dist = sdf_ellipse(p_cx, p_cy, p_rx, p_ry, x, y);
      let dist_to_band = dist.abs() - half;
      // Simple linear AA over ~1.0 pixel
      let coverage = ((1.0 - dist_to_band).clamp(0.0, 1.0)) * hardness;
      if coverage <= 0.0 {
        continue;
      }

      let alpha = alpha_base * coverage;

      let idx = ((yy * img_w + xx) as usize) * 4;
      let dst_r = arr[idx] as f32;
      let dst_g = arr[idx + 1] as f32;
      let dst_b = arr[idx + 2] as f32;
      let dst_a = arr[idx + 3] as f32 / 255.0;

      // Source-over composite: out = src*alpha + dst*(1-src_alpha)
      let out_a = alpha + dst_a * (1.0 - alpha);
      let out_r = (avg_r * alpha + dst_r * (1.0 - alpha)).round();
      let out_g = (avg_g * alpha + dst_g * (1.0 - alpha)).round();
      let out_b = (avg_b * alpha + dst_b * (1.0 - alpha)).round();
      let out_a_byte = (out_a * 255.0).round() as u8;

      arr[idx] = out_r as u8;
      arr[idx + 1] = out_g as u8;
      arr[idx + 2] = out_b as u8;
      arr[idx + 3] = out_a_byte;
    }
  }
}

/// Fill a closed `Area` using SDF polygon evaluation and optional feathering.
/// - `feather` is the feather radius in pixels; a value of 0 performs a hard fill.
pub fn draw_area_fill(p_img: &mut Image, p_area: &abra_core::Area, p_brush: &crate::Brush) {
  // Compute bounds and adaptive tolerance before flattening
  let (min_x_f, min_y_f, max_x_f, max_y_f) = p_area.path.bounds();
  let area_w = (max_x_f - min_x_f).max(max_y_f - min_y_f);
  // tolerance scales with size: larger areas can use looser flattening
  let tol = (area_w * 0.005).max(0.5).min(8.0);

  let pts = p_area.path.flatten(tol);
  if pts.is_empty() {
    return;
  }

  // If shape looks like an ellipse, use analytic fast-path
  if let Some((cx, cy, rx, ry)) = detect_ellipse_from_points(&pts, 0.06) {
    draw_ellipse_fill(p_img, cx, cy, rx, ry, p_brush);
    return;
  }

  // feather comes from area.feather (if set) otherwise 0
  let feather = p_area.feather() as f32;

  let min_x = ((min_x_f - feather - 1.0).floor() as i32).max(0) as u32;
  let max_x = ((max_x_f + feather + 1.0).ceil() as i32).min(p_img.dimensions::<u32>().0 as i32 - 1) as u32;
  let min_y = ((min_y_f - feather - 1.0).floor() as i32).max(0) as u32;
  let max_y = ((max_y_f + feather + 1.0).ceil() as i32).min(p_img.dimensions::<u32>().1 as i32 - 1) as u32;

  let (img_w, _img_h) = p_img.dimensions::<u32>();
  let arr = p_img.colors();

  // Extract color and opacity from brush
  let (color, opacity) = match p_brush.color() {
    abra_core::Fill::Solid(c) => (c, p_brush.opacity()),
    _ => return,
  };
  let avg_r = color.r as f32;
  let avg_g = color.g as f32;
  let avg_b = color.b as f32;
  let alpha_base = (color.a as f32 / 255.0) * opacity;

  for yy in min_y..=max_y {
    let y = yy as f32 + 0.5;
    for xx in min_x..=max_x {
      let x = xx as f32 + 0.5;
      let dist = sdf_polygon(&pts, x, y);

      let coverage = if feather > 0.0 {
        // Linear ramp across feather distance: inside -> 1, outside beyond feather -> 0
        ((feather - dist).clamp(0.0, feather) / feather).clamp(0.0, 1.0)
      } else {
        if dist <= 0.0 { 1.0 } else { 0.0 }
      };

      if coverage <= 0.0 {
        continue;
      }
      let alpha = alpha_base * coverage;

      let idx = ((yy * img_w + xx) as usize) * 4;
      let dst_r = arr[idx] as f32;
      let dst_g = arr[idx + 1] as f32;
      let dst_b = arr[idx + 2] as f32;
      let dst_a = arr[idx + 3] as f32 / 255.0;

      let out_a = alpha + dst_a * (1.0 - alpha);
      let out_r = (avg_r * alpha + dst_r * (1.0 - alpha)).round();
      let out_g = (avg_g * alpha + dst_g * (1.0 - alpha)).round();
      let out_b = (avg_b * alpha + dst_b * (1.0 - alpha)).round();
      let out_a_byte = (out_a * 255.0).round() as u8;

      arr[idx] = out_r as u8;
      arr[idx + 1] = out_g as u8;
      arr[idx + 2] = out_b as u8;
      arr[idx + 3] = out_a_byte;
    }
  }
}

/// Stroke an `Area` by painting a band around its outline using SDF polygon distance.
/// `stroke_w` is total width and `hardness` (0..1) scales the coverage falloff.
pub fn draw_area_stroke(p_img: &mut Image, p_area: &abra_core::Area, p_brush: &crate::Brush) {
  // Compute bounds first and pick adaptive flatten tolerance
  let (min_x_f, min_y_f, max_x_f, max_y_f) = p_area.path.bounds();
  let area_w = (max_x_f - min_x_f).max(max_y_f - min_y_f);
  let tol = (area_w * 0.005).max(0.5).min(8.0);

  let pts = p_area.path.flatten(tol);
  if pts.is_empty() {
    return;
  }

  // If the area is actually an ellipse, use the analytic ellipse stroke fast-path
  if let Some((cx, cy, rx, ry)) = detect_ellipse_from_points(&pts, 0.06) {
    draw_ellipse_stroke(p_img, cx, cy, rx, ry, p_brush);
    return;
  }

  // Extract stroke width, color and hardness from brush (only Solid supported)
  let stroke_w = p_brush.size() as f32;
  let half = stroke_w / 2.0;
  let (color, hardness, opacity) = match p_brush.color() {
    abra_core::Fill::Solid(c) => (c, p_brush.hardness(), p_brush.opacity()),
    _ => return,
  };

  let (min_x, min_y, max_x, max_y) = p_area.path.bounds();
  let min_x = ((min_x - half - 1.0).floor() as i32).max(0) as u32;
  let max_x = ((max_x + half + 1.0).ceil() as i32).min(p_img.dimensions::<u32>().0 as i32 - 1) as u32;
  let min_y = ((min_y - half - 1.0).floor() as i32).max(0) as u32;
  let max_y = ((max_y + half + 1.0).ceil() as i32).min(p_img.dimensions::<u32>().1 as i32 - 1) as u32;

  let (img_w, _img_h) = p_img.dimensions::<u32>();
  let arr = p_img.colors();
  let avg_r = color.r as f32;
  let avg_g = color.g as f32;
  let avg_b = color.b as f32;
  let alpha_base = (color.a as f32 / 255.0) * opacity;

  for yy in min_y..=max_y {
    let y = yy as f32 + 0.5;
    for xx in min_x..=max_x {
      let x = xx as f32 + 0.5;
      let dist = sdf_polygon(&pts, x, y);
      let dist_to_band = dist.abs() - half;
      let coverage = ((1.0 - dist_to_band).clamp(0.0, 1.0)) * hardness;
      if coverage <= 0.0 {
        continue;
      }

      let alpha = alpha_base * coverage;
      let idx = ((yy * img_w + xx) as usize) * 4;
      let dst_r = arr[idx] as f32;
      let dst_g = arr[idx + 1] as f32;
      let dst_b = arr[idx + 2] as f32;
      let dst_a = arr[idx + 3] as f32 / 255.0;

      let out_a = alpha + dst_a * (1.0 - alpha);
      let out_r = (avg_r * alpha + dst_r * (1.0 - alpha)).round();
      let out_g = (avg_g * alpha + dst_g * (1.0 - alpha)).round();
      let out_b = (avg_b * alpha + dst_b * (1.0 - alpha)).round();
      let out_a_byte = (out_a * 255.0).round() as u8;

      arr[idx] = out_r as u8;
      arr[idx + 1] = out_g as u8;
      arr[idx + 2] = out_b as u8;
      arr[idx + 3] = out_a_byte;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Image;

  #[test]
  fn sdf_ellipse_basic() {
    let d = sdf_ellipse(0.0, 0.0, 10.0, 5.0, 10.0, 0.0);
    assert!(d.abs() < 1.0);
  }

  #[test]
  fn draw_ellipse_stroke_writes_pixels() {
    let mut img = Image::new(200, 200);
    let red = abra_core::Color::from_rgba(255, 0, 0, 255);
    let brush = crate::Brush::new().with_size(10).with_color(&red).with_hardness(1.0);
    draw_ellipse_stroke(&mut img, 100.0, 100.0, 80.0, 80.0, &brush);
    let rgba = img.rgba();
    // Check a pixel near the expected ring location is non-zero alpha
    let idx = ((100 * 200 + 180) as usize) * 4 + 3;
    assert!(rgba[idx] != 0, "expected non-zero alpha at ring pixel");
  }

  #[test]
  fn draw_area_fill_and_stroke() {
    let mut img = Image::new(200, 200);
    let area = abra_core::Area::ellipse((100.0, 100.0), (120.0, 120.0));
    let green = abra_core::Color::from_rgba(0, 255, 0, 255);
    let red = abra_core::Color::from_rgba(255, 0, 0, 255);
    let fill_brush = crate::Brush::new().with_color(&green).with_opacity(1.0);
    let stroke_brush = crate::Brush::new().with_size(10).with_color(&red).with_hardness(1.0);
    draw_area_fill(&mut img, &area, &fill_brush);
    draw_area_stroke(&mut img, &area, &stroke_brush);
    let rgba = img.rgba();
    // sample a pixel inside the filled area
    let idx_inside = ((100 * 200 + 100) as usize) * 4 + 3;
    assert!(rgba[idx_inside] != 0, "expected fill to write alpha");
    // sample a pixel near the stroke
    let idx_stroke = ((100 * 200 + 160) as usize) * 4 + 3;
    assert!(rgba[idx_stroke] != 0, "expected stroke to write alpha");
  }
}
