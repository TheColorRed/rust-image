//! The Oklab color space, and Oklch, its polar form.
//!
//! Oklab (Björn Ottosson, 2020) is built so that equal steps look equal: its lightness `L` follows how bright a color looks, and its two
//! color axes `a` and `b` follow how red-green and yellow-blue it looks. Mixing in it avoids the dull middles of mixing in sRGB, and
//! turning the hue in its polar form Oklch does not change how bright the color looks, which turning the hue in HSV does.

use super::to_lab::srgb_u8_to_linear_f32;
use super::to_rgb::linear_to_srgb;

/// Converts linear-light RGB (0-1) to Oklab. The result is `(L, a, b)`: `L` is 0 for black and 1 for white, and `a` and `b` are about
/// -0.4 to 0.4.
pub(crate) fn linear_rgb_to_oklab(p_r: f32, p_g: f32, p_b: f32) -> (f32, f32, f32) {
  let l = (0.4122214708 * p_r + 0.5363325363 * p_g + 0.0514459929 * p_b).cbrt();
  let m = (0.2119034982 * p_r + 0.6806995451 * p_g + 0.1073969566 * p_b).cbrt();
  let s = (0.0883024619 * p_r + 0.2817188376 * p_g + 0.6299787005 * p_b).cbrt();
  (
    0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
  )
}

/// Converts Oklab to linear-light RGB, without clamping: a color outside the sRGB gamut has a channel below 0 or above 1.
pub(crate) fn oklab_to_linear_rgb(p_l: f32, p_a: f32, p_b: f32) -> (f32, f32, f32) {
  let l = (p_l + 0.3963377774 * p_a + 0.2158037573 * p_b).powi(3);
  let m = (p_l - 0.1055613458 * p_a - 0.0638541728 * p_b).powi(3);
  let s = (p_l - 0.0894841775 * p_a - 1.2914855480 * p_b).powi(3);
  (
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
  )
}

/// Converts sRGB color to Oklab.
/// - `p_r`: The red channel (0-255).
/// - `p_g`: The green channel (0-255).
/// - `p_b`: The blue channel (0-255).
/// Returns a tuple `(L, a, b)` with `L` from 0 (black) to 1 (white).
pub fn rgb_to_oklab(p_r: u8, p_g: u8, p_b: u8) -> (f32, f32, f32) {
  linear_rgb_to_oklab(srgb_u8_to_linear_f32(p_r), srgb_u8_to_linear_f32(p_g), srgb_u8_to_linear_f32(p_b))
}

/// Converts Oklab color to sRGB. A color outside the sRGB gamut is clipped to it, channel by channel.
/// - `p_l`: The lightness (0-1).
/// - `p_a`: The red-green axis (about -0.4 to 0.4).
/// - `p_b`: The yellow-blue axis (about -0.4 to 0.4).
/// Returns a tuple `(R, G, B)` representing the sRGB color.
pub fn oklab_to_rgb(p_l: f32, p_a: f32, p_b: f32) -> (u8, u8, u8) {
  let (r, g, b) = oklab_to_linear_rgb(p_l, p_a, p_b);
  let byte = |channel: f32| (linear_to_srgb(channel).clamp(0.0, 1.0) * 255.0).round() as u8;
  (byte(r), byte(g), byte(b))
}

/// Converts a rectangular color (`a` and `b` axes) to its polar form: `(L, chroma, hue)` with the hue in degrees from 0 to 360. The
/// hue of a color with no chroma is 0.
pub(crate) fn to_polar(p_l: f32, p_a: f32, p_b: f32) -> (f32, f32, f32) {
  let chroma = (p_a * p_a + p_b * p_b).sqrt();
  let hue = p_b.atan2(p_a).to_degrees().rem_euclid(360.0);
  (p_l, chroma, if chroma == 0.0 { 0.0 } else { hue })
}

/// Converts a polar color `(L, chroma, hue)`, the hue in degrees, back to its rectangular form `(L, a, b)`.
pub(crate) fn from_polar(p_l: f32, p_chroma: f32, p_hue: f32) -> (f32, f32, f32) {
  let (sin, cos) = p_hue.to_radians().sin_cos();
  (p_l, p_chroma * cos, p_chroma * sin)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn near(p_a: f32, p_b: f32, p_tolerance: f32) -> bool {
    (p_a - p_b).abs() <= p_tolerance
  }

  #[test]
  fn black_and_white_and_grays_have_no_color() {
    let (l, a, b) = rgb_to_oklab(0, 0, 0);
    assert!(near(l, 0.0, 1e-5) && near(a, 0.0, 1e-5) && near(b, 0.0, 1e-5));
    let (l, a, b) = rgb_to_oklab(255, 255, 255);
    assert!(near(l, 1.0, 1e-4) && near(a, 0.0, 1e-4) && near(b, 0.0, 1e-4), "{l} {a} {b}");
    let (_, a, b) = rgb_to_oklab(120, 120, 120);
    assert!(near(a, 0.0, 1e-4) && near(b, 0.0, 1e-4), "{a} {b}");
  }

  #[test]
  fn the_primaries_have_the_values_the_reference_gives() {
    // From Björn Ottosson's reference implementation.
    let (l, a, b) = rgb_to_oklab(255, 0, 0);
    assert!(near(l, 0.6280, 1e-3) && near(a, 0.2249, 1e-3) && near(b, 0.1258, 1e-3), "red {l} {a} {b}");
    let (l, a, b) = rgb_to_oklab(0, 255, 0);
    assert!(near(l, 0.8664, 1e-3) && near(a, -0.2339, 1e-3) && near(b, 0.1795, 1e-3), "green {l} {a} {b}");
    let (l, a, b) = rgb_to_oklab(0, 0, 255);
    assert!(near(l, 0.4520, 1e-3) && near(a, -0.0325, 1e-3) && near(b, -0.3115, 1e-3), "blue {l} {a} {b}");
  }

  #[test]
  fn every_color_comes_back_to_itself() {
    for r in (0..=255).step_by(15) {
      for g in (0..=255).step_by(15) {
        for b in (0..=255).step_by(15) {
          let (l, a, ab) = rgb_to_oklab(r as u8, g as u8, b as u8);
          assert_eq!(oklab_to_rgb(l, a, ab), (r as u8, g as u8, b as u8));
        }
      }
    }
  }

  #[test]
  fn a_color_outside_the_gamut_is_clipped() {
    // Far more colorful than sRGB can show.
    let (r, g, b) = oklab_to_rgb(0.7, 0.4, 0.3);
    assert_eq!(r, 255);
    assert!(g < 255 && b < 255);
    assert_eq!(oklab_to_rgb(1.5, 0.0, 0.0), (255, 255, 255));
    assert_eq!(oklab_to_rgb(-0.5, 0.0, 0.0), (0, 0, 0));
  }

  #[test]
  fn polar_form_round_trips_and_a_gray_has_hue_zero() {
    let (l, a, b) = rgb_to_oklab(220, 60, 90);
    let (l2, chroma, hue) = to_polar(l, a, b);
    let (l3, a3, b3) = from_polar(l2, chroma, hue);
    assert!(near(l, l3, 1e-5) && near(a, a3, 1e-5) && near(b, b3, 1e-5));
    assert!((0.0..360.0).contains(&hue));
    assert_eq!(to_polar(0.5, 0.0, 0.0), (0.5, 0.0, 0.0));
    // Red is around 29 degrees in Oklch.
    let (_, _, red_hue) = {
      let (l, a, b) = rgb_to_oklab(255, 0, 0);
      to_polar(l, a, b)
    };
    assert!(near(red_hue, 29.2, 0.5), "{red_hue}");
  }
}
