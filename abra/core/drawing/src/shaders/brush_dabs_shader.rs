use crate::Shader;
use abra_core::PointF;
use std::collections::HashMap;

/// A shader that paints multiple brush dabs in a single pass.
///
/// - `inner`: underlying color shader (solid/gradient/image)
/// - `centers`: list of dab center positions
/// - `max_distance`: radius for dab influence
/// - `hardness`: falloff hardness
pub(crate) struct BrushDabsShader {
  inner: Box<dyn Shader + Send + Sync>,
  centers_by_cell: HashMap<(i32, i32), Vec<PointF>>,
  max_distance: f32,
  max_distance_sq: f32,
  hardness: f32,
}

impl BrushDabsShader {
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_centers: Vec<PointF>, p_max_distance: f32, p_hardness: f32,
  ) -> Self {
    let max_distance = p_max_distance.max(f32::EPSILON);
    let mut centers_by_cell: HashMap<(i32, i32), Vec<PointF>> = HashMap::new();
    for center in p_centers {
      let cell = Self::cell_for(center.x, center.y, max_distance);
      centers_by_cell.entry(cell).or_default().push(center);
    }

    BrushDabsShader {
      inner: p_inner,
      centers_by_cell,
      max_distance,
      max_distance_sq: max_distance * max_distance,
      hardness: p_hardness.clamp(0.0, 1.0),
    }
  }

  fn cell_for(p_x: f32, p_y: f32, p_cell_size: f32) -> (i32, i32) {
    ((p_x / p_cell_size).floor() as i32, (p_y / p_cell_size).floor() as i32)
  }

  // compute alpha falloff based on distance^2 (avoid sqrt inside loops by using squared distances)
  fn compute_alpha_falloff(&self, p_dist_sq: f32) -> f32 {
    if p_dist_sq >= self.max_distance_sq {
      return 0.0;
    }
    let normalized_sq = p_dist_sq / self.max_distance_sq;
    // We need a function that behaves similarly to the original falloff.
    // Convert squared normalized distance back to normalized distance for falloff curve.
    let normalized = normalized_sq.sqrt();
    if self.hardness < 0.5 {
      let t = self.hardness * 2.0;
      let quadratic = 1.0 - (normalized * normalized);
      let linear = 1.0 - normalized;
      quadratic * (1.0 - t) + linear * t
    } else {
      let t = (self.hardness - 0.5) * 2.0;
      let linear = 1.0 - normalized;
      let hard_edge = if normalized < 0.5 { 1.0 } else { 0.0 };
      linear * (1.0 - t) + hard_edge * t
    }
    .max(0.0)
  }
}

impl Shader for BrushDabsShader {
  fn shade(&self, p_x: f32, p_y: f32) -> (u8, u8, u8, u8) {
    let mut r_acc = 0.0f32;
    let mut g_acc = 0.0f32;
    let mut b_acc = 0.0f32;
    let mut a_acc = 0.0f32;

    let cell = Self::cell_for(p_x, p_y, self.max_distance);
    let mut inner_color = None;
    for cell_y in cell.1 - 1..=cell.1 + 1 {
      for cell_x in cell.0 - 1..=cell.0 + 1 {
        let Some(centers) = self.centers_by_cell.get(&(cell_x, cell_y)) else {
          continue;
        };
        for center in centers {
          let dx = p_x - center.x;
          let dy = p_y - center.y;
          let falloff = self.compute_alpha_falloff(dx * dx + dy * dy);
          if falloff <= 0.0 {
            continue;
          }

          let (ir, ig, ib, ia) = *inner_color.get_or_insert_with(|| self.inner.shade(p_x, p_y));
          let fa = (ia as f32) * falloff;
          r_acc += (ir as f32) * fa / 255.0;
          g_acc += (ig as f32) * fa / 255.0;
          b_acc += (ib as f32) * fa / 255.0;
          a_acc += fa;
        }
      }
    }

    let out_a = a_acc.min(255.0) as u8;
    if a_acc > 0.0 {
      let inv_a = 255.0 / a_acc;
      let out_r = (r_acc * inv_a).min(255.0) as u8;
      let out_g = (g_acc * inv_a).min(255.0) as u8;
      let out_b = (b_acc * inv_a).min(255.0) as u8;
      (out_r, out_g, out_b, out_a)
    } else {
      (0, 0, 0, 0)
    }
  }
}
