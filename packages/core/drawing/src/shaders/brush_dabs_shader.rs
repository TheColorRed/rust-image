use crate::Shader;
use core::PointF;

/// A shader that paints multiple brush dabs in a single pass.
///
/// - `inner`: underlying color shader (solid/gradient/image)
/// - `centers`: list of dab center positions
/// - `max_distance`: radius for dab influence
/// - `hardness`: falloff hardness
pub(crate) struct BrushDabsShader {
  inner: Box<dyn Shader + Send + Sync>,
  centers: Vec<PointF>,
  max_distance: f32,
  hardness: f32,
}

impl BrushDabsShader {
  pub fn new(
    p_inner: Box<dyn Shader + Send + Sync>, p_centers: Vec<PointF>, p_max_distance: f32, p_hardness: f32,
  ) -> Self {
    BrushDabsShader {
      inner: p_inner,
      centers: p_centers,
      max_distance: p_max_distance,
      hardness: p_hardness.clamp(0.0, 1.0),
    }
  }

  // compute alpha falloff based on distance^2 (avoid sqrt inside loops by using squared distances)
  fn compute_alpha_falloff(&self, dist_sq: f32) -> f32 {
    let max_d_sq = self.max_distance * self.max_distance;
    if dist_sq >= max_d_sq {
      return 0.0;
    }
    let normalized_sq = dist_sq / max_d_sq;
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
    // For each center, compute alpha falloff and sample inner shader at the given coordinate.
    // We composite contributions additively here and clamp.
    let mut r_acc = 0.0f32;
    let mut g_acc = 0.0f32;
    let mut b_acc = 0.0f32;
    let mut a_acc = 0.0f32;

    // Compute the inner shader color once per sample (it's independent of center)
    let (ir, ig, ib, ia) = self.inner.shade(p_x, p_y);
    for center in &self.centers {
      let dx = p_x - center.x;
      let dy = p_y - center.y;
      let dist_sq = dx * dx + dy * dy;
      let falloff = self.compute_alpha_falloff(dist_sq);
      if falloff <= 0.0 {
        continue;
      }
      let fa = (ia as f32) * falloff;
      let fr = (ir as f32) * fa / 255.0;
      let fg = (ig as f32) * fa / 255.0;
      let fb = (ib as f32) * fa / 255.0;
      r_acc += fr;
      g_acc += fg;
      b_acc += fb;
      a_acc += fa;
    }

    // Convert back to u8, applying min clamps.
    let out_a = a_acc.min(255.0) as u8;
    // Avoid division by zero; premultiply if we had alpha accumulation.
    if a_acc > 0.0 {
      // Un-premultiply by alpha to get final channels in 0-255 space.
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
