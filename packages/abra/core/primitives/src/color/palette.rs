//! Palettes: a list of colors, and ready made ones.
//!
//! A [`Palette`] answers "which of these colors is nearest to this one?" by how different the colors look (distance in the Lab color
//! space), so a photo's colors can be snapped to a set of inks, a screen's colors or a theme.

use super::Color;

/// A list of colors to choose from, with the nearest of them to any color. Create one from colors with [`Palette::new`], or take a
/// ready made one such as [`Palette::comic`].
///
/// Colors are compared by how different they look: the distance between their Lab values, which is what the eye sees much better than
/// the distance between red, green and blue values. Alpha is not looked at, and a color given back is the palette's own, alpha
/// included.
///
/// ```ignore
/// let inks = Palette::silk_screen();
/// let ink = inks.nearest(Color::from_rgb(180, 90, 40)); // the silk screen color nearest to this brown
/// ```
#[derive(Clone, Debug)]
pub struct Palette {
  colors: Vec<Color>,
  /// The Lab values of each color, worked out once so that finding the nearest color for many pixels does not do it again.
  labs: Vec<[f32; 3]>,
}

impl Palette {
  /// Creates a palette of these colors, in this order.
  pub fn new(p_colors: impl IntoIterator<Item = impl Into<Color>>) -> Self {
    let colors: Vec<Color> = p_colors.into_iter().map(Into::into).collect();
    let labs = colors
      .iter()
      .map(|color| {
        let (l, a, b) = color.lab();
        [l, a, b]
      })
      .collect();
    Self { colors, labs }
  }

  /// The colors of the palette, in order.
  pub fn colors(&self) -> &[Color] {
    &self.colors
  }

  /// The Lab values of the colors of the palette, in order.
  pub fn labs(&self) -> &[[f32; 3]] {
    &self.labs
  }

  /// How many colors the palette has.
  pub fn len(&self) -> usize {
    self.colors.len()
  }

  /// Whether the palette has no colors.
  pub fn is_empty(&self) -> bool {
    self.colors.is_empty()
  }

  /// How different two colors look, as the square of the distance between their Lab values. Compare it with another of its results,
  /// or take the square root for a distance in which about 2 is the least a person can see and 100 is black against white.
  pub fn distance(p_first: &[f32; 3], p_second: &[f32; 3]) -> f32 {
    let l = p_first[0] - p_second[0];
    let a = p_first[1] - p_second[1];
    let b = p_first[2] - p_second[2];
    l * l + a * a + b * b
  }

  /// The place in the palette of the color nearest to a color, or `None` when the palette has no colors. The first of two colors
  /// that are equally near wins.
  pub fn nearest_index(&self, p_color: impl Into<Color>) -> Option<usize> {
    let (l, a, b) = p_color.into().lab();
    self.nearest_index_to_lab([l, a, b])
  }

  /// Like [`Palette::nearest_index`] for a color whose Lab values are already worked out, as when going over the pixels of an image.
  pub fn nearest_index_to_lab(&self, p_lab: [f32; 3]) -> Option<usize> {
    Self::nearest_in(&self.labs, &p_lab)
  }

  /// The place of the Lab color of a list that is nearest to another Lab color, or `None` for an empty list. The first of two that are
  /// equally near wins. It is the search a palette does, for a list of Lab colors that is not a palette, such as the centers of groups.
  pub fn nearest_in(p_labs: &[[f32; 3]], p_lab: &[f32; 3]) -> Option<usize> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;
    for (index, lab) in p_labs.iter().enumerate() {
      let distance = Self::distance(lab, p_lab);
      if distance < nearest_distance {
        nearest_distance = distance;
        nearest = Some(index);
      }
    }
    nearest
  }

  /// The color of the palette nearest to a color, or `None` when the palette has no colors.
  pub fn nearest(&self, p_color: impl Into<Color>) -> Option<Color> {
    self.nearest_index(p_color).map(|index| self.colors[index])
  }

  /// Each color of the palette as `p_steps` colors of its own hue, from dark to light: the shades and tints of one ink, with the ink
  /// itself as one of them. So a palette of 8 inks with 8 steps is a palette of 64 colors. The steps go from near black to near white,
  /// evenly in the Lab color space, and the step nearest to a color's own brightness is the color itself, exactly. The steps either side
  /// of it keep its hue and lose color the further they are from it, down to a little under half at the dark and light ends, where the
  /// colors of a screen cannot have much. A color that is bright, such as a yellow, therefore has more shades than tints, and a dark
  /// one such as a navy has more tints than shades.
  /// - `p_steps`: How many colors each color becomes. A `p_steps` of 0 is taken as 1, which is the color itself.
  pub fn tones(&self, p_steps: usize) -> Self {
    let steps = p_steps.max(1);
    // The lightness of a step, from the darkest to the lightest. The only step of one is in the middle.
    let lightness_of = move |step: usize| {
      let across = if steps == 1 { 0.5 } else { step as f32 / (steps - 1) as f32 };
      DARKEST + (LIGHTEST - DARKEST) * across
    };
    Self::new(self.colors.iter().zip(&self.labs).flat_map(|(color, lab)| {
      let (color, lab) = (*color, *lab);
      // The step nearest to the color's own brightness is the color.
      let own = (0..steps)
        .min_by(|&first, &second| (lightness_of(first) - lab[0]).abs().total_cmp(&(lightness_of(second) - lab[0]).abs()))
        .unwrap_or(0);
      (0..steps).map(move |step| {
        if step == own {
          return color;
        }
        let lightness = lightness_of(step);
        // How far the step is from the color, from 0 at the color to 1 at the end of the range on its side.
        let away = if lightness >= lab[0] {
          (lightness - lab[0]) / (LIGHTEST - lab[0]).max(1.0)
        } else {
          (lab[0] - lightness) / (lab[0] - DARKEST).max(1.0)
        };
        let chroma = 1.0 - 0.55 * away.clamp(0.0, 1.0).powi(2);
        Color::from_lab(lightness, lab[1] * chroma, lab[2] * chroma)
      })
    }))
  }

  /// `p_levels` grays from black to white, evenly spaced. A `p_levels` of 0 is taken as 1, which is black.
  pub fn grayscale(p_levels: usize) -> Self {
    let levels = p_levels.max(1);
    Self::new((0..levels).map(|level| {
      let gray = if levels == 1 { 0 } else { (level * 255 / (levels - 1)) as u8 };
      Color::from_rgb(gray, gray, gray)
    }))
  }

  /// The inks of a comic book, 64 colors: red, magenta, orange, yellow, lime, green, teal, cyan, blue and a peach for skin, each in 5
  /// tones from dark to light (see [`Palette::tones`], the ink itself is one of them), and 14 grays from black to white for the page.
  pub fn comic() -> Self {
    let inks = Self::new(
      [
        (230, 30, 40),
        (236, 0, 140),
        (250, 120, 30),
        (255, 214, 0),
        (140, 200, 40),
        (30, 150, 70),
        (0, 170, 170),
        (0, 174, 239),
        (20, 60, 200),
        (255, 190, 150),
      ]
      .map(rgb),
    )
    .tones(5);
    Self::new(inks.into_iter().chain(Self::grayscale(14)))
  }

  /// The colors of a silk screen print, 64 colors: navy, hot pink, yellow, turquoise, a red-orange, violet, green and a gray, each in 8
  /// tones from dark to light (see [`Palette::tones`], the ink itself is one of them).
  pub fn silk_screen() -> Self {
    Self::new(
      [(24, 20, 84), (255, 40, 140), (255, 214, 0), (30, 190, 200), (240, 70, 50), (140, 60, 200), (60, 190, 90), (128, 128, 128)]
        .map(rgb),
    )
    .tones(8)
  }

  /// Bright colors that glow against a dark background: hot pink, cyan, green, yellow and purple, on near black.
  pub fn neon() -> Self {
    Self::new([(255, 20, 147), (0, 255, 255), (57, 255, 20), (255, 240, 0), (180, 0, 255), (10, 10, 30)].map(rgb))
  }

  /// Soft light colors: pink, peach, yellow, mint, sky blue and lavender, and white.
  pub fn pastel() -> Self {
    Self::new(
      [(255, 179, 186), (255, 223, 186), (255, 255, 186), (186, 255, 201), (186, 225, 255), (220, 200, 255), (255, 255, 255)].map(rgb),
    )
  }

  /// Royal blue against gold with muted companions, 40 colors: a bright blue, a warm yellow, a dusty periwinkle, a khaki and a
  /// blue-gray, each in 8 tones from dark to light (see [`Palette::tones`], the ink itself is one of them).
  pub fn dusk() -> Self {
    Self::new([(0x4B, 0x75, 0xFF), (0xFF, 0xCE, 0x4A), (0x67, 0x7C, 0xBF), (0xAA, 0x99, 0x6A), (0x65, 0x6B, 0x80)].map(rgb)).tones(8)
  }

  /// The four colors of the first color graphics cards for the PC (CGA, the high intensity palette): black, cyan, magenta and white.
  pub fn cga() -> Self {
    Self::new([(0, 0, 0), (85, 255, 255), (255, 85, 255), (255, 255, 255)].map(rgb))
  }

  /// The four greens of the screen of the original Game Boy, from darkest to lightest.
  pub fn game_boy() -> Self {
    Self::new([(15, 56, 15), (48, 98, 48), (139, 172, 15), (155, 188, 15)].map(rgb))
  }

  /// 64 browns from dark to light, as an old photograph is printed.
  pub fn sepia() -> Self {
    Self::new([(176, 125, 70)].map(rgb)).tones(64)
  }
}

/// The lightness, in the Lab color space, of the darkest tone of [`Palette::tones`].
const DARKEST: f32 = 12.0;
/// The lightness, in the Lab color space, of the lightest tone of [`Palette::tones`].
const LIGHTEST: f32 = 95.0;

fn rgb((p_red, p_green, p_blue): (u8, u8, u8)) -> Color {
  Color::from_rgb(p_red, p_green, p_blue)
}

impl IntoIterator for Palette {
  type Item = Color;
  type IntoIter = std::vec::IntoIter<Color>;

  fn into_iter(self) -> Self::IntoIter {
    self.colors.into_iter()
  }
}

impl<'a> IntoIterator for &'a Palette {
  type Item = Color;
  type IntoIter = std::iter::Copied<std::slice::Iter<'a, Color>>;

  fn into_iter(self) -> Self::IntoIter {
    self.colors.iter().copied()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_nearest_color_is_the_one_that_looks_most_like_it() {
    let palette = Palette::new([Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255), Color::from_rgb(255, 255, 255)]);
    assert_eq!(palette.nearest(Color::from_rgb(200, 60, 40)), Some(Color::from_rgb(255, 0, 0)));
    assert_eq!(palette.nearest_index(Color::from_rgb(30, 30, 160)), Some(1));
    assert_eq!(palette.nearest_index(Color::from_rgb(240, 240, 250)), Some(2));
    // A palette color is nearest to itself.
    for (index, color) in palette.colors().iter().enumerate() {
      assert_eq!(palette.nearest_index(*color), Some(index));
    }
  }

  #[test]
  fn an_empty_palette_has_no_nearest_color_and_the_first_of_equals_wins() {
    let empty = Palette::new(Vec::<Color>::new());
    assert!(empty.is_empty());
    assert_eq!(empty.nearest(Color::red()), None);

    let twins = Palette::new([Color::from_rgb(10, 200, 10), Color::from_rgb(10, 200, 10)]);
    assert_eq!(twins.nearest_index(Color::from_rgb(10, 200, 10)), Some(0));
  }

  #[test]
  fn the_given_alpha_is_the_palette_colors_and_not_the_searched_colors() {
    let palette = Palette::new([Color::from_rgba(0, 255, 0, 128)]);
    assert_eq!(palette.nearest(Color::from_rgba(0, 250, 0, 255)).map(|color| color.rgba()), Some((0, 255, 0, 128)));
  }

  #[test]
  fn the_ready_made_palettes_have_no_twin_colors() {
    let all = [
      Palette::comic(),
      Palette::silk_screen(),
      Palette::neon(),
      Palette::pastel(),
      Palette::dusk(),
      Palette::cga(),
      Palette::game_boy(),
      Palette::sepia(),
    ];
    for palette in all {
      assert!(palette.len() >= 4);
      // Every color is far enough from the others that it is the nearest color of itself and not a twin of another.
      for (index, color) in palette.colors().iter().enumerate() {
        assert_eq!(palette.nearest_index(*color), Some(index));
      }
      // No two are the same color. A fine ramp, such as the sepia, may have neighbors too near to tell apart by eye, but not twins.
      let distinct: std::collections::HashSet<(u8, u8, u8)> = palette.colors().iter().map(|color| color.rgb()).collect();
      assert_eq!(distinct.len(), palette.len());
    }
  }

  #[test]
  fn tones_make_each_color_a_row_from_dark_to_light_of_its_own_hue() {
    let blue = Palette::new([Color::from_rgb(20, 40, 150)]).tones(8);
    assert_eq!(blue.len(), 8);
    // The brightness goes up from one tone to the next, and the hue stays blue (the Lab b value stays below zero).
    let lightness: Vec<f32> = blue.labs().iter().map(|lab| lab[0]).collect();
    assert!(lightness.windows(2).all(|pair| pair[0] < pair[1]), "{lightness:?}");
    assert!(blue.labs().iter().all(|lab| lab[2] < 0.0));
    // The color itself is one of the tones, exactly.
    assert!(blue.colors().contains(&Color::from_rgb(20, 40, 150)));
    // Two colors with three tones each are six colors, the first color's tones first.
    let two = Palette::new([Color::from_rgb(200, 40, 40), Color::from_rgb(40, 160, 60)]).tones(3);
    assert_eq!(two.len(), 6);
    assert!(two.labs()[0][1] > 0.0 && two.labs()[5][1] < 0.0, "red tones have a positive a, green ones a negative a");
    // No steps is taken as one, which is the color itself.
    assert_eq!(Palette::new([Color::red()]).tones(0).colors(), &[Color::red()]);
    // A bright color has more shades than tints, and a dark one more tints than shades.
    let yellow: Vec<f32> = Palette::new([Color::from_rgb(255, 214, 0)]).tones(8).labs().iter().map(|lab| lab[0]).collect();
    let navy: Vec<f32> = Palette::new([Color::from_rgb(24, 20, 84)]).tones(8).labs().iter().map(|lab| lab[0]).collect();
    assert!(yellow.iter().filter(|&&l| l < 86.0).count() > yellow.iter().filter(|&&l| l > 86.0).count());
    assert!(navy.iter().filter(|&&l| l > 14.0).count() > navy.iter().filter(|&&l| l < 14.0).count());
  }

  #[test]
  fn a_grayscale_palette_has_evenly_spaced_grays() {
    let grays = Palette::grayscale(5);
    let values: Vec<u8> = grays.colors().iter().map(|color| color.rgb().0).collect();
    assert_eq!(values, vec![0, 63, 127, 191, 255]);
    assert_eq!(Palette::grayscale(0).len(), 1);
  }

  #[test]
  fn the_big_palettes_have_sixty_four_colors() {
    assert_eq!(Palette::comic().len(), 64);
    assert_eq!(Palette::silk_screen().len(), 64);
    assert_eq!(Palette::sepia().len(), 64);
  }

  #[test]
  fn a_palette_can_be_walked_as_colors() {
    let palette = Palette::cga();
    let by_reference: Vec<Color> = (&palette).into_iter().collect();
    let owned: Vec<Color> = palette.into_iter().collect();
    assert_eq!(by_reference.len(), 4);
    assert_eq!(owned.len(), 4);
  }
}
