use crate::Resolution;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Units {
  /// The value is specified in pixels.
  Pixels(u32),
  /// The value is specified in inches.
  Inches(u32),
  /// The value is specified in centimeters.
  Centimeters(u32),
  /// The value is specified in millimeters.
  Millimeters(u32),
  /// The value is specified in points, where 72 points equal one inch.
  Points(u32),
  /// The value is specified in picas, where 6 picas equal one inch.
  Picas(u32),
  /// The value is specified as a percentage of a reference length in pixels.
  ///
  /// The percentage is resolved against the reference length supplied to [`Units::to_pixels`].
  Percent(u32),
}

impl Units {
  /// Converts this value to pixels.
  /// - `p_reference`: The length in pixels that 100% represents. Only used by [`Units::Percent`], which returns
  ///   `p_reference * value / 100` (e.g. `Percent(50)` with a reference of `200.0` is `100.0`). Ignored by all
  ///   other units.
  /// - `p_resolution`: The pixel density used to convert physical units. Only its horizontal density is used, so a
  ///   value is the same length in every direction.
  pub fn to_pixels(self, p_reference: f32, p_resolution: Resolution) -> f32 {
    let dpi = p_resolution.x_dpi;
    match self {
      Self::Pixels(value) => value as f32,
      Self::Inches(value) => value as f32 * dpi,
      Self::Centimeters(value) => value as f32 / 2.54 * dpi,
      Self::Millimeters(value) => value as f32 / 25.4 * dpi,
      Self::Points(value) => value as f32 / 72.0 * dpi,
      Self::Picas(value) => value as f32 / 6.0 * dpi,
      Self::Percent(value) => p_reference * value as f32 / 100.0,
    }
  }
}
