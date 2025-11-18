//! Compositing operations for blending colors.

/// Trait for compositing source color onto destination.
pub(crate) trait Compositor: Sync {
  /// Composites source (r, g, b, a) with coverage onto destination pixel.
  /// Returns the final (r, g, b, a) to write.
  fn composite(
    &self, p_src_r: u8, p_src_g: u8, p_src_b: u8, p_src_a: u8, p_coverage: f32, p_dst_r: u8, p_dst_g: u8, p_dst_b: u8,
    p_dst_a: u8,
  ) -> (u8, u8, u8, u8);
}

/// Source-over compositing with coverage-based alpha.
pub(crate) struct SourceOverCompositor;

impl Compositor for SourceOverCompositor {
  fn composite(
    &self, p_src_r: u8, p_src_g: u8, p_src_b: u8, p_src_a: u8, p_coverage: f32, p_dst_r: u8, p_dst_g: u8, p_dst_b: u8,
    p_dst_a: u8,
  ) -> (u8, u8, u8, u8) {
    let src_alpha = (p_src_a as f32 / 255.0) * p_coverage;
    let dst_r = p_dst_r as f32;
    let dst_g = p_dst_g as f32;
    let dst_b = p_dst_b as f32;
    let dst_a = p_dst_a as f32 / 255.0;

    // Source-over: out = src * src_a + dst * dst_a * (1 - src_a)
    let out_a = src_alpha + dst_a * (1.0 - src_alpha);
    let (out_r, out_g, out_b) = if out_a > 0.0 {
      let out_r = ((p_src_r as f32 * src_alpha) + (dst_r * dst_a * (1.0 - src_alpha))) / out_a;
      let out_g = ((p_src_g as f32 * src_alpha) + (dst_g * dst_a * (1.0 - src_alpha))) / out_a;
      let out_b = ((p_src_b as f32 * src_alpha) + (dst_b * dst_a * (1.0 - src_alpha))) / out_a;
      (out_r, out_g, out_b)
    } else {
      (0.0, 0.0, 0.0)
    };

    (
      out_r.round().clamp(0.0, 255.0) as u8,
      out_g.round().clamp(0.0, 255.0) as u8,
      out_b.round().clamp(0.0, 255.0) as u8,
      (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    )
  }
}

/// Overwrite compositing (ignores destination; used for gradient legacy behavior).
pub(crate) struct OverwriteCompositor;

impl Compositor for OverwriteCompositor {
  fn composite(
    &self, p_src_r: u8, p_src_g: u8, p_src_b: u8, p_src_a: u8, p_coverage: f32, _p_dst_r: u8, _p_dst_g: u8,
    _p_dst_b: u8, _p_dst_a: u8,
  ) -> (u8, u8, u8, u8) {
    let alpha = (p_src_a as f32 / 255.0) * p_coverage;
    (p_src_r, p_src_g, p_src_b, (alpha * 255.0).round().clamp(0.0, 255.0) as u8)
  }
}
