/// Physical pixel density metadata for an image or document.
///
/// Resolution changes physical output size metadata only; it never resizes pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolution {
  pub x_dpi: f32,
  pub y_dpi: f32,
}

impl Resolution {
  /// Standard screen resolution.
  pub const SCREEN: Self = Self::new(96.0, 96.0);

  /// Common print resolution.
  pub const PRINT: Self = Self::new(300.0, 300.0);

  /// Common banner resolution.
  pub const BANNER: Self = Self::new(150.0, 150.0);

  /// Common billboard resolution.
  pub const BILLBOARD: Self = Self::new(50.0, 50.0);

  /// Common art resolution.
  pub const ART: Self = Self::new(1200.0, 1200.0);

  /// Common line art resolution.
  pub const LINE_ART: Self = Self::new(600.0, 600.0);

  pub const fn new(p_x_dpi: f32, p_y_dpi: f32) -> Self {
    Self {
      x_dpi: p_x_dpi,
      y_dpi: p_y_dpi,
    }
  }

  pub const fn uniform(p_dpi: f32) -> Self {
    Self::new(p_dpi, p_dpi)
  }
}

impl Default for Resolution {
  fn default() -> Self {
    Self::SCREEN
  }
}
