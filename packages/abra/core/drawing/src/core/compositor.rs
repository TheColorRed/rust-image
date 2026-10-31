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

use abra_core::BlendMode;

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

/// How a drawn color is combined with what is already under it: any of abra's blend modes, or `Additive`.
///
/// It is a [`Compositor`] itself, so it can be handed to the rasterizer as it is, and a [`Brush`](crate::Brush) carries one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Blend {
  /// Adds the drawn color's light to what is there, scaled by its opacity: `dst + src * opacity`. Overlaps get brighter and nothing
  /// darkens, as with sparks, fire and glows. It is not [`BlendMode::LinearDodge`], which mixes toward the sum by the opacity and so
  /// cannot add more light than a bright pixel has room for.
  Additive,
  /// One of abra's blend modes, with the opacity mixing the blended color in. `Normal` is plain source-over painting.
  Mode(BlendMode),
}

impl Default for Blend {
  fn default() -> Self {
    Blend::Mode(BlendMode::Normal)
  }
}

impl From<BlendMode> for Blend {
  fn from(p_mode: BlendMode) -> Self {
    Blend::Mode(p_mode)
  }
}

impl Compositor for Blend {
  fn composite(
    &self, p_src_r: u8, p_src_g: u8, p_src_b: u8, p_src_a: u8, p_coverage: f32, p_dst_r: u8, p_dst_g: u8, p_dst_b: u8,
    p_dst_a: u8,
  ) -> (u8, u8, u8, u8) {
    let mode = match self {
      Blend::Mode(BlendMode::Normal) => {
        return SourceOverCompositor.composite(p_src_r, p_src_g, p_src_b, p_src_a, p_coverage, p_dst_r, p_dst_g, p_dst_b, p_dst_a);
      }
      Blend::Mode(mode) => Some(mode),
      Blend::Additive => None,
    };
    // The alpha is worked out as source-over does, and the color is the blend of the two, mixed in by the source's opacity.
    let src_alpha = (p_src_a as f32 / 255.0) * p_coverage;
    let out_alpha = src_alpha + (p_dst_a as f32 / 255.0) * (1.0 - src_alpha);
    let blended = mode.map(|mode| mode.apply((p_dst_r, p_dst_g, p_dst_b, p_dst_a), (p_src_r, p_src_g, p_src_b, 255)));
    let channel = |dst: u8, src: u8, blended: Option<u8>| -> u8 {
      let value = match blended {
        None => dst as f32 + src as f32 * src_alpha,
        Some(blended) => dst as f32 + (blended as f32 - dst as f32) * src_alpha,
      };
      value.round().clamp(0.0, 255.0) as u8
    };
    (
      channel(p_dst_r, p_src_r, blended.map(|b| b.0)),
      channel(p_dst_g, p_src_g, blended.map(|b| b.1)),
      channel(p_dst_b, p_src_b, blended.map(|b| b.2)),
      (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Paints a color with full coverage onto a pixel with the given blend.
  fn paint(p_blend: Blend, p_src: (u8, u8, u8, u8), p_dst: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
    p_blend.composite(p_src.0, p_src.1, p_src.2, p_src.3, 1.0, p_dst.0, p_dst.1, p_dst.2, p_dst.3)
  }

  #[test]
  fn normal_is_source_over() {
    let (src, dst) = ((200, 100, 50, 128), (10, 20, 30, 255));
    assert_eq!(
      paint(Blend::default(), src, dst),
      SourceOverCompositor.composite(src.0, src.1, src.2, src.3, 1.0, dst.0, dst.1, dst.2, dst.3)
    );
  }

  #[test]
  fn additive_adds_the_light_scaled_by_the_opacity_and_stops_at_white() {
    // Half opaque white onto 100 adds about 128, and onto a bright pixel it stops at 255.
    assert_eq!(paint(Blend::Additive, (255, 255, 255, 128), (100, 100, 100, 255)), (228, 228, 228, 255));
    assert_eq!(paint(Blend::Additive, (255, 255, 255, 255), (200, 200, 200, 255)).0, 255);
    // Nothing darkens, and a black source adds nothing.
    assert_eq!(paint(Blend::Additive, (0, 0, 0, 255), (90, 80, 70, 255)), (90, 80, 70, 255));
  }

  #[test]
  fn a_blend_mode_mixes_the_blend_in_by_the_opacity() {
    // Multiply by mid gray halves the destination; at half opacity it goes half way there.
    let full = paint(Blend::Mode(BlendMode::Multiply), (128, 128, 128, 255), (200, 200, 200, 255));
    let half = paint(Blend::Mode(BlendMode::Multiply), (128, 128, 128, 128), (200, 200, 200, 255));
    assert!((99..=101).contains(&full.0));
    assert!(half.0 > full.0 && half.0 < 200);
  }
}
