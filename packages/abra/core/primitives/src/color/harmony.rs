use super::Color;

/// A color harmony (color scheme) that can be generated from a source color.
///
/// Every scheme returned by [`Color::harmony`] includes the source color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Harmony {
  /// Five neighboring hues at -30, -15, 0, 15, and 30 degrees, with the source in the middle.
  /// Low contrast and cohesive.
  Analogous,
  /// The source and the hue 180 degrees opposite it. The strongest hue contrast.
  Complementary,
  /// Five colors in Adobe-style theme order: the source, the hues on either side of its complement
  /// (+210 and +150 degrees), then darker variants of the source and of the +210 hue.
  SplitComplementary,
  /// The source and the hues 120 and 240 degrees away.
  Triadic,
  /// The source and the hues 90, 180, and 270 degrees away.
  Square,
  /// The source, a neighboring hue at 30 degrees, and that neighbor's opposite at 210 degrees.
  Compound,
  /// The given number of progressively darker shades, starting at the source and ending at 75% black.
  Shades(usize),
  /// The given number of lightness variants of the source's hue and saturation, with the source in the middle.
  Monochromatic(usize),
}

impl Color {
  /// Generates a color scheme from this color.
  ///
  /// Saturation and alpha are preserved unless the scheme itself varies them.
  ///
  /// - `p_harmony`: The scheme to generate.
  ///
  /// ```ignore
  /// let palette = Color::red().harmony(Harmony::Triadic); // [red, green, blue]
  /// ```
  pub fn harmony(&self, p_harmony: Harmony) -> Vec<Color> {
    match p_harmony {
      Harmony::Analogous => self.rotate_hues(&[-30.0, -15.0, 0.0, 15.0, 30.0]),
      Harmony::Complementary => self.rotate_hues(&[0.0, 180.0]),
      Harmony::SplitComplementary => self.split_complementary(),
      Harmony::Triadic => self.rotate_hues(&[0.0, 120.0, 240.0]),
      Harmony::Square => self.rotate_hues(&[0.0, 90.0, 180.0, 270.0]),
      Harmony::Compound => self.rotate_hues(&[0.0, 30.0, 210.0]),
      Harmony::Shades(count) => self.shades(count),
      Harmony::Monochromatic(count) => self.monochromatic(count),
    }
  }

  /// Returns this color with its HSL hue rotated by each offset (degrees).
  fn rotate_hues(&self, p_offsets: &[f32]) -> Vec<Color> {
    let (h, s, l) = self.hsl();
    p_offsets
      .iter()
      .map(|offset| Color {
        a: self.a,
        ..Color::from_hsl((h + offset).rem_euclid(360.0), s, l)
      })
      .collect()
  }

  fn split_complementary(&self) -> Vec<Color> {
    let (h, s, v) = self.hsv();
    let hsv = |h: f32, s: f32, v: f32| Color {
      a: self.a,
      ..Color::from_hsv(h.rem_euclid(360.0), s, v)
    };
    vec![
      *self,
      hsv(h + 210.0, s, v),
      hsv(h + 150.0, s, v),
      hsv(h, s * 0.5, v * 0.5),
      hsv(h + 210.0, s * 0.5, v * 0.5),
    ]
  }

  fn shades(&self, p_count: usize) -> Vec<Color> {
    (0..p_count)
      .map(|index| {
        let progress = if p_count == 1 { 0.0 } else { index as f32 / (p_count - 1) as f32 };
        let scale = 1.0 - progress * 0.75;
        let channel = |value: u8| (value as f32 * scale).round() as u8;
        Color::from_rgba(channel(self.r), channel(self.g), channel(self.b), self.a)
      })
      .collect()
  }

  fn monochromatic(&self, p_count: usize) -> Vec<Color> {
    if p_count == 0 {
      return Vec::new();
    }
    let (h, s, l) = self.hsl();
    let with_l = |l: f32| Color {
      a: self.a,
      ..Color::from_hsl(h, s, l)
    };
    let darker_count = p_count / 2;
    let lighter_count = p_count - darker_count - 1;
    let darker = (1..=darker_count).map(|index| with_l(l * index as f32 / (darker_count + 1) as f32));
    let lighter = (1..=lighter_count).map(|index| with_l(l + (1.0 - l) * index as f32 / (lighter_count + 1) as f32));
    darker.chain(std::iter::once(*self)).chain(lighter).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn analogous_green_stays_between_chartreuse_and_mint() {
    let colors: Vec<_> = Color::green().harmony(Harmony::Analogous).into_iter().map(|color| color.rgb()).collect();
    assert_eq!(colors, vec![(128, 255, 0), (64, 255, 0), (0, 255, 0), (0, 255, 64), (0, 255, 128)]);
  }

  #[test]
  fn schemes_start_with_or_center_the_source() {
    let red = Color::red();
    assert_eq!(red.harmony(Harmony::Complementary)[1].rgb(), (0, 255, 255));
    let triadic: Vec<_> = red.harmony(Harmony::Triadic).iter().map(|color| color.rgb()).collect();
    assert_eq!(triadic, vec![(255, 0, 0), (0, 255, 0), (0, 0, 255)]);
    assert_eq!(red.harmony(Harmony::Square).len(), 4);
    assert_eq!(red.harmony(Harmony::Compound)[0], red);
    assert_eq!(red.harmony(Harmony::Analogous)[2], red);
  }

  #[test]
  fn harmony_colors_preserve_alpha() {
    let source = Color::from_rgba(255, 0, 0, 73);
    for harmony in [
      Harmony::Analogous,
      Harmony::Complementary,
      Harmony::SplitComplementary,
      Harmony::Triadic,
      Harmony::Square,
      Harmony::Compound,
      Harmony::Shades(4),
      Harmony::Monochromatic(4),
    ] {
      assert!(source.harmony(harmony).iter().all(|color| color.a == source.a), "{harmony:?}");
    }
    assert_eq!(source.harmony(Harmony::SplitComplementary).len(), 5);
  }

  #[test]
  fn shades_mix_the_source_evenly_toward_black() {
    let source = Color::from_rgba(120, 255, 60, 73);
    let shades = source.harmony(Harmony::Shades(5));
    assert_eq!(shades.len(), 5);
    assert_eq!(shades[0], source);
    assert_eq!(shades[4].rgba(), (30, 64, 15, 73));
    assert!(shades.windows(2).all(|pair| pair[0].luminance() > pair[1].luminance()));
  }

  #[test]
  fn monochromatic_colors_surround_and_include_source() {
    let source = Color::from_rgba(0, 255, 0, 73);
    let colors = source.harmony(Harmony::Monochromatic(5));
    assert_eq!(colors.len(), 5);
    assert_eq!(colors[2], source);
    assert!(colors[0].hsl().2 < source.hsl().2);
    assert!(colors[4].hsl().2 > source.hsl().2);
  }

  #[test]
  fn zero_count_schemes_are_empty() {
    assert!(Color::red().harmony(Harmony::Shades(0)).is_empty());
    assert!(Color::red().harmony(Harmony::Monochromatic(0)).is_empty());
  }
}
