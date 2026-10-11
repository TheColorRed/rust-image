//! Named color constructors.
//!
//! Every zero-argument public constructor returning `Self` in this file is also exported over FFI
//! as `abra_color_<name>`: the `abra` crate's build script (`build_ffi/color_list.rs`) scans this
//! file's source text to generate those bindings. Keep only zero-argument constructors here, and
//! do not write constructor signatures in comments (the scan would pick them up).

use super::Color;

impl Color {
  /// A transparent color using RGBA(0, 0, 0, 0)
  pub fn transparent() -> Self {
    Self::from_rgba(0, 0, 0, 0)
  }
  /// Black color using RGB(0, 0, 0)
  pub fn black() -> Self {
    Self::from_rgb(0, 0, 0)
  }
  /// White color using RGB(255, 255, 255)
  pub fn white() -> Self {
    Self::from_rgb(255, 255, 255)
  }
  /// Gray color using RGB(128, 128, 128)
  pub fn gray() -> Self {
    Self::from_rgb(128, 128, 128)
  }
  /// Red color using RGB(255, 0, 0)
  pub fn red() -> Self {
    Self::from_rgb(255, 0, 0)
  }
  /// Crimson color using RGB(220, 20, 60)
  pub fn crimson() -> Self {
    Self::from_rgb(220, 20, 60)
  }
  /// Ruby color using RGB(224, 17, 95)
  pub fn ruby() -> Self {
    Self::from_rgb(224, 17, 95)
  }
  /// Pink color using RGB(255, 192, 203)
  pub fn pink() -> Self {
    Self::from_rgb(255, 192, 203)
  }
  /// Magenta color using RGB(255, 0, 255)
  pub fn magenta() -> Self {
    Self::from_rgb(255, 0, 255)
  }
  /// Hot pink color using RGB(255, 105, 180)
  pub fn hot_pink() -> Self {
    Self::from_rgb(255, 105, 180)
  }
  /// Green color using RGB(0, 255, 0)
  pub fn green() -> Self {
    Self::from_rgb(0, 255, 0)
  }
  /// Lime green color using RGB(50, 205, 50)
  pub fn lime_green() -> Self {
    Self::from_rgb(50, 205, 50)
  }
  /// Sea green color using RGB(46, 139, 87)
  pub fn sea_green() -> Self {
    Self::from_rgb(46, 139, 87)
  }
  /// Forest green color using RGB(34, 139, 34)
  pub fn forest_green() -> Self {
    Self::from_rgb(34, 139, 34)
  }
  /// Blue color using RGB(0, 0, 255)
  pub fn blue() -> Self {
    Self::from_rgb(0, 0, 255)
  }
  /// Royal blue color using RGB(65, 105, 225)
  pub fn royal_blue() -> Self {
    Self::from_rgb(65, 105, 225)
  }
  /// Sky blue color using RGB(135, 206, 235)
  pub fn sky_blue() -> Self {
    Self::from_rgb(135, 206, 235)
  }
  /// Navy blue color using RGB(0, 0, 128)
  pub fn navy_blue() -> Self {
    Self::from_rgb(0, 0, 128)
  }
  /// Yellow color using RGB(255, 255, 0)
  pub fn yellow() -> Self {
    Self::from_rgb(255, 255, 0)
  }
  /// Gold color using RGB(255, 215, 0)
  pub fn gold() -> Self {
    Self::from_rgb(255, 215, 0)
  }
  /// Golden color using RGB(255, 223, 0)
  pub fn golden() -> Self {
    Self::from_rgb(255, 223, 0)
  }
  /// Bronze color using RGB(205, 127, 50)
  pub fn bronze() -> Self {
    Self::from_rgb(205, 127, 50)
  }
  /// Orange color using RGB(255, 165, 0)
  pub fn orange() -> Self {
    Self::from_rgb(255, 165, 0)
  }
  /// Indigo color using RGB(75, 0, 130)
  pub fn indigo() -> Self {
    Self::from_rgb(75, 0, 130)
  }
  /// Violet color using RGB(238, 130, 238)
  pub fn violet() -> Self {
    Self::from_rgb(238, 130, 238)
  }
  /// Purple color using RGB(128, 0, 128)
  pub fn purple() -> Self {
    Self::from_rgb(128, 0, 128)
  }
  /// Tan color using RGB(210, 180, 140)
  pub fn tan() -> Self {
    Self::from_rgb(210, 180, 140)
  }
  /// Beige color using RGB(245, 245, 220)
  pub fn beige() -> Self {
    Self::from_rgb(245, 245, 220)
  }
  /// Brown color using RGB(165, 42, 42)
  pub fn brown() -> Self {
    Self::from_rgb(139, 90, 43)
  }
  /// Dark brown color using RGB(101, 67, 33)
  pub fn dark_brown() -> Self {
    Self::from_rgb(101, 67, 33)
  }
  /// Light brown color using RGB(181, 101, 29)
  pub fn light_brown() -> Self {
    Self::from_rgb(181, 101, 29)
  }
  /// Random opaque color.
  pub fn random() -> Self {
    // Lightweight LCG seeded from current system time to avoid adding rand dependency.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
    let mut x: u64 = nanos.wrapping_mul(6364136223846793005).wrapping_add(1);
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
    let r = (x >> 24) as u8;
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
    let g = (x >> 32) as u8;
    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
    let b = (x >> 16) as u8;
    Self::from_rgb(r, g, b)
  }
}
