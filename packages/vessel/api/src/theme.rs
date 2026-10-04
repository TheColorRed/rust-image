//! Colors and shapes that components take their look from, inherited like CSS custom properties.
//!
//! A [`Theme`] is the full set of values. The host supplies one where the app says nothing (a desktop window gives the
//! user's system theme: dark or light, and their accent color), and any component can change parts of it for everything
//! inside it with a [`ThemePatch`]:
//!
//! ```ignore
//! page.set_theme(Theme::dark());                                      // a whole theme for this part of the app
//! sidebar.set_theme(ThemePatch::new().accent([200, 40, 90, 255]));    // just the accent, and just in the sidebar
//! ```
//!
//! Each value inherits on its own, so a patch that sets only the accent keeps everything else from above. A component's
//! `draw` reads the result from its canvas, as `canvas.theme()`. Individual components can still override one thing, for
//! example `button.set_color(..)`.

/// Makes `Theme`, `ThemePatch` and the methods that go with them from one list of values, so a new value is added in one
/// place.
macro_rules! theme_values {
  ($($(#[$about:meta])* $name:ident: $type:ty,)*) => {
    /// Every value a component can take its look from.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Theme {
      $($(#[$about])* pub $name: $type,)*
    }

    /// Some of a theme's values: the ones a component sets for itself and what is inside it. What it leaves out comes
    /// from above. Build one with [`ThemePatch::new`] and a method for each value, or turn a whole [`Theme`] into one.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct ThemePatch {
      $($(#[$about])* pub $name: Option<$type>,)*
    }

    impl ThemePatch {
      /// A patch that sets nothing.
      pub fn new() -> Self {
        Self::default()
      }

      $(
        $(#[$about])*
        pub fn $name(mut self, p_value: $type) -> Self {
          self.$name = Some(p_value);
          self
        }
      )*

      /// This patch, with anything it leaves out taken from `p_other`.
      pub(crate) fn or(&self, p_other: &ThemePatch) -> ThemePatch {
        ThemePatch { $($name: self.$name.clone().or_else(|| p_other.$name.clone()),)* }
      }

      /// A full theme: this patch's values, and `p_base`'s for the rest.
      pub(crate) fn over(&self, p_base: Theme) -> Theme {
        Theme { $($name: self.$name.clone().unwrap_or(p_base.$name),)* }
      }
    }

    impl From<Theme> for ThemePatch {
      fn from(p_theme: Theme) -> Self {
        Self { $($name: Some(p_theme.$name),)* }
      }
    }
  };
}

theme_values! {
  /// The color of the page behind everything.
  background: [u8; 4],
  /// The color of raised areas such as a panel or a card.
  surface: [u8; 4],
  /// The color of ordinary text.
  text: [u8; 4],
  /// The color of less important text.
  muted_text: [u8; 4],
  /// The face of a plain control such as a button.
  control: [u8; 4],
  /// The color of text on a plain control. Like a browser's `buttontext`, it is not the page's text color: changing
  /// `text` does not change it.
  control_text: [u8; 4],
  /// The thin outline around a plain control.
  border: [u8; 4],
  /// The main color of things you can act on: a button, the filled part of a slider.
  accent: [u8; 4],
  /// The color of text on top of the accent.
  accent_text: [u8; 4],
  /// The color of an empty track or an outline.
  track: [u8; 4],
  /// How much corners are rounded, in pixels.
  radius: f32,
}

impl Theme {
  /// The light theme with the standard blue accent.
  pub fn light() -> Self {
    Self {
      background: [243, 243, 243, 255],
      surface: [255, 255, 255, 255],
      text: [27, 27, 27, 255],
      muted_text: [96, 96, 96, 255],
      control: [239, 239, 239, 255],
      control_text: [27, 27, 27, 255],
      border: [118, 118, 118, 255],
      accent: [0, 103, 192, 255],
      accent_text: [255, 255, 255, 255],
      track: [230, 230, 230, 255],
      radius: 4.0,
    }
  }

  /// The dark theme with the standard blue accent.
  pub fn dark() -> Self {
    Self {
      background: [32, 32, 32, 255],
      surface: [44, 44, 44, 255],
      text: [255, 255, 255, 255],
      muted_text: [170, 170, 170, 255],
      control: [59, 59, 59, 255],
      control_text: [255, 255, 255, 255],
      border: [133, 133, 133, 255],
      accent: [76, 194, 255, 255],
      accent_text: [0, 0, 0, 255],
      track: [100, 100, 100, 255],
      radius: 4.0,
    }
  }

  /// This theme with the accent changed to `p_rgb`. The text on it turns black or white, whichever is easier to read.
  pub fn with_accent(mut self, p_rgb: [u8; 3]) -> Self {
    self.accent = [p_rgb[0], p_rgb[1], p_rgb[2], 255];
    let brightness = (p_rgb[0] as u32 * 299 + p_rgb[1] as u32 * 587 + p_rgb[2] as u32 * 114) / 1000;
    self.accent_text = if brightness > 150 { [0, 0, 0, 255] } else { [255, 255, 255, 255] };
    self
  }

  /// The user's own theme where the platform can tell: dark or light apps and their accent color. This is the theme a
  /// desktop window gives its component. Where the platform cannot say (or the `desktop-window` flag is off) it is
  /// [`light`](Self::light).
  pub fn system() -> Self {
    #[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
    {
      let appearance = vessel_engine::surface::desktop::appearance();
      let theme = if appearance.dark { Self::dark() } else { Self::light() };
      return match appearance.accent {
        Some(accent) => theme.with_accent(accent),
        None => theme,
      };
    }
    #[cfg(not(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios")))))]
    Self::light()
  }
}

impl Default for Theme {
  /// The light theme.
  fn default() -> Self {
    Self::light()
  }
}

/// What a component fills its whole area with before it draws, like CSS `background`. Not inherited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Background {
  /// Nothing: whatever is behind shows through.
  #[default]
  None,
  /// This color.
  Color([u8; 4]),
  /// The theme's page color, so the area follows the theme.
  Page,
  /// The theme's surface color, so the area follows the theme.
  Surface,
  /// The face color of a plain control, so the area follows the theme.
  Control,
  /// The accent color, so the area follows the theme.
  Accent,
}

impl Background {
  /// The color to fill with under `p_theme`, if any.
  pub(crate) fn resolve(&self, p_theme: &Theme) -> Option<[u8; 4]> {
    match self {
      Self::None => None,
      Self::Color(color) => Some(*color),
      Self::Page => Some(p_theme.background),
      Self::Surface => Some(p_theme.surface),
      Self::Control => Some(p_theme.control),
      Self::Accent => Some(p_theme.accent),
    }
  }
}

/// How much a component's corners are rounded, like CSS `border-radius`. Not inherited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Radius {
  /// This many pixels (0 is square). Half the shorter side makes a pill or a circle.
  Px(f32),
  /// The theme's radius, so the corners follow the theme.
  Theme,
}

impl Default for Radius {
  /// Square corners.
  fn default() -> Self {
    Self::Px(0.0)
  }
}

impl From<f32> for Radius {
  fn from(p_pixels: f32) -> Self {
    Self::Px(p_pixels)
  }
}

impl Radius {
  /// The radius in pixels under `p_theme`.
  pub(crate) fn resolve(&self, p_theme: &Theme) -> f32 {
    match self {
      Self::Px(pixels) => *pixels,
      Self::Theme => p_theme.radius,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_patch_replaces_only_what_it_sets() {
    let patch = ThemePatch::new().accent([1, 2, 3, 255]).radius(0.0);
    let theme = patch.over(Theme::dark());
    assert_eq!(theme.accent, [1, 2, 3, 255]);
    assert_eq!(theme.radius, 0.0);
    assert_eq!(theme.text, Theme::dark().text, "the rest is untouched");
  }

  #[test]
  fn a_patch_takes_what_it_leaves_out_from_another_and_a_whole_theme_sets_everything() {
    let near = ThemePatch::new().accent([9, 9, 9, 255]);
    let far = ThemePatch::new().accent([1, 1, 1, 255]).radius(2.0);
    let merged = near.or(&far);
    assert_eq!((merged.accent, merged.radius), (Some([9, 9, 9, 255]), Some(2.0)));
    assert_eq!(merged.text, None);

    assert_eq!(ThemePatch::from(Theme::light()).over(Theme::dark()), Theme::light());
  }

  #[test]
  fn the_accents_text_is_readable_on_it() {
    assert_eq!(Theme::light().with_accent([255, 230, 0]).accent_text, [0, 0, 0, 255], "dark text on a bright accent");
    assert_eq!(Theme::dark().with_accent([10, 30, 120]).accent_text, [255, 255, 255, 255], "light text on a dark one");
  }

  #[test]
  fn backgrounds_follow_the_theme_unless_they_are_a_fixed_color() {
    let theme = Theme::dark();
    assert_eq!(Background::None.resolve(&theme), None);
    assert_eq!(Background::Page.resolve(&theme), Some(theme.background));
    assert_eq!(Background::Surface.resolve(&theme), Some(theme.surface));
    assert_eq!(Background::Color([5, 6, 7, 8]).resolve(&theme), Some([5, 6, 7, 8]));
  }

  #[test]
  fn light_is_the_default_and_the_system_theme_is_always_available() {
    assert_eq!(Theme::default(), Theme::light());
    let system = Theme::system();
    assert_eq!(system.accent[3], 255, "the accent is opaque");
  }
}
