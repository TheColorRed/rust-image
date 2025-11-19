//! Compositing operations used to merge per-sample source colors into
//! destination pixel values.
//!
//! The rasterizer gathers sample colors for a pixel and averages them; a
//! `Compositor` determines how this averaged source color should be
//! blended with the already-written destination pixel. This module
//! provides the `Compositor` trait and a few implementations, including
//! a source-over compositing rule and a simple overwrite compositor.
//!
//! Key points
//! - Compositors are responsible for combining source RGBA with a
//!   destination RGBA and a coverage factor in `[0.0,1.0]` to compute a
//!   final RGBA value for the pixel.
//! - Implementations must be `Sync` (they are called by the parallel
//!   rasterization code).
//! - Core implementations: `SourceOverCompositor` (standard alpha
//!   compositing) and `OverwriteCompositor` (replace destination with
//!   source, used for legacy behavior or special effects).
//!
//! Example
//! ```ignore
//! let comp = SourceOverCompositor;
//! let out = comp.composite(255, 0, 0, 128, 1.0, 0, 0, 255, 255);
//! ```

/// Trait for compositing a source color on top of a destination color.
///
/// Implementations provide a `composite` method that computes an output
/// RGBA value using the source RGBA, a coverage amount in [0.0, 1.0], and
/// the destination RGBA value. This abstraction allows the rasterizer to
/// apply different blending rules such as source-over or direct overwrite
/// without changing the sampling or shading logic.
///
/// Parameters
/// - `p_src_r`, `p_src_g`, `p_src_b`, `p_src_a`: Source color components (0-255).
/// - `p_coverage`: Fraction of the sample covered by the source (0.0-1.0).
/// - `p_dst_r`, `p_dst_g`, `p_dst_b`, `p_dst_a`: Destination pixel components.
///
/// Returns the computed `(r, g, b, a)` tuple (each 0-255) to write to the
/// destination pixel.
pub trait Compositor: Sync {
  /// Composites source (r, g, b, a) with coverage onto destination pixel.
  /// Returns the final (r, g, b, a) to write.
  fn composite(
    &self, p_src_r: u8, p_src_g: u8, p_src_b: u8, p_src_a: u8, p_coverage: f32, p_dst_r: u8, p_dst_g: u8, p_dst_b: u8,
    p_dst_a: u8,
  ) -> (u8, u8, u8, u8);
}

/// Source-over compositing with coverage-based alpha.
///
/// The source-over rule is the classical alpha compositing mode from Porter
/// and Duff. The final pixel color is computed as:
/// out = src * src_a + dst * dst_a * (1 - src_a)
///
/// Example
/// ```ignore
/// let compositor = SourceOverCompositor;
/// let out = compositor.composite(255, 0, 0, 128, 1.0, 0, 0, 255, 255);
/// // out now contains the blended RGBA value.
/// ```
pub struct SourceOverCompositor;

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
