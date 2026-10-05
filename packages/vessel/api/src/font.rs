use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Horizontal alignment of text inside its content box, inherited by text descendants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
  /// Align to the left edge.
  #[default]
  Left,
  /// Center horizontally.
  Center,
  /// Align to the right edge.
  Right,
}

/// A typeface: the shapes of the letters. Either the small built-in bitmap face, which needs nothing, or an outline face
/// loaded from a font file, which scales smoothly and has letters of different widths.
///
/// Where a component's typeface comes from is the host's business: a desktop window uses the system's, and a view inside
/// another application uses whatever that application tells it to (see `Component::set_default_font`).
#[derive(Clone)]
pub struct Face(Kind);

#[derive(Clone)]
enum Kind {
  Bitmap,
  Outline(Arc<fontdue::Font>),
}

impl Face {
  /// The built-in 8x8 bitmap face for plain ASCII. It is always available, and is what text uses until a host says
  /// otherwise.
  pub fn bitmap() -> Self {
    Self(Kind::Bitmap)
  }

  /// A face from the bytes of a TrueType or OpenType font file, or `None` if they are not a font it can read.
  pub fn from_bytes(p_bytes: &[u8]) -> Option<Self> {
    fontdue::Font::from_bytes(p_bytes, fontdue::FontSettings::default())
      .ok()
      .map(|font| Self(Kind::Outline(Arc::new(font))))
  }

  /// A face from a font file, or `None` if the file is missing or is not a font it can read.
  pub fn from_file(p_path: impl AsRef<Path>) -> Option<Self> {
    Self::from_bytes(&std::fs::read(p_path).ok()?)
  }

  /// The operating system's usual interface font (Segoe UI on Windows, Roboto on Android, and so on), or the built-in
  /// bitmap face if none of the usual places has one. It is looked up once.
  pub fn system() -> Self {
    static SYSTEM: OnceLock<Face> = OnceLock::new();
    SYSTEM
      .get_or_init(|| {
        const CANDIDATES: &[&str] = &[
          "C:\\Windows\\Fonts\\segoeui.ttf",
          "/system/fonts/Roboto-Regular.ttf",
          "/system/fonts/Roboto[wdth,wght].ttf",
          "/system/fonts/DroidSans.ttf",
          "/System/Library/Fonts/Supplemental/Arial.ttf",
          "/Library/Fonts/Arial.ttf",
          "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
          "/usr/share/fonts/TTF/DejaVuSans.ttf",
          "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        ];
        CANDIDATES.iter().find_map(Self::from_file).unwrap_or_else(Self::bitmap)
      })
      .clone()
  }

  /// Whether this is an outline face (loaded from a font file) rather than the built-in bitmap.
  pub fn is_outline(&self) -> bool {
    matches!(self.0, Kind::Outline(_))
  }

  /// The outline font, if there is one.
  pub(crate) fn outline(&self) -> Option<&fontdue::Font> {
    match &self.0 {
      Kind::Outline(font) => Some(font),
      Kind::Bitmap => None,
    }
  }
}

impl PartialEq for Face {
  fn eq(&self, p_other: &Self) -> bool {
    match (&self.0, &p_other.0) {
      (Kind::Bitmap, Kind::Bitmap) => true,
      (Kind::Outline(a), Kind::Outline(b)) => Arc::ptr_eq(a, b),
      _ => false,
    }
  }
}

impl std::fmt::Debug for Face {
  fn fmt(&self, p_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    p_formatter.write_str(if self.is_outline() { "Face(outline)" } else { "Face(bitmap)" })
  }
}

/// How text is drawn: a typeface and a size in pixels.
///
/// A font is inherited, like CSS, and its two parts inherit separately: set the size on a component with
/// `set_font_size` and the typeface with `set_font_face`, and every component inside it that has not set its own uses
/// them. Whatever nobody sets comes from the host (a desktop window uses the system's face), and failing that the
/// built-in 16 pixel bitmap. A component's `draw` reads the font it ended up with from its canvas.
///
/// ```ignore
/// page.set_font_size(24);                      // everything in the page
/// small_print.set_font_size(8);                // except this one
/// small_print.set_font_size(Units::Percent(50.0)); // half the parent's computed font size
/// heading.set_font_size(Units::Em(1.5));       // 1.5 times the inherited size
/// page.set_font_face(Face::from_file("my.ttf").unwrap());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
  /// The typeface.
  pub face: Face,
  /// The height of a line of text in pixels.
  pub size: u32,
}

impl Font {
  /// A font in `p_face` whose text is `p_size` pixels tall. The built-in bitmap is drawn at the nearest whole multiple of
  /// 8 pixels (at least 8).
  pub fn new(p_face: Face, p_size: u32) -> Self {
    Self {
      face: p_face,
      size: p_size,
    }
  }

  /// The system's interface font at `p_size` pixels. This is what a desktop window gives its component.
  pub fn system(p_size: u32) -> Self {
    Self::new(Face::system(), p_size)
  }

  /// How many times the 8 pixel bitmap letters are enlarged to reach this size.
  pub(crate) fn bitmap_scale(&self) -> u32 {
    ((self.size + 4) / 8).max(1)
  }
}

impl Default for Font {
  /// The built-in bitmap face at 16 pixels.
  fn default() -> Self {
    Self::new(Face::bitmap(), 16)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_bitmap_scale_is_the_nearest_whole_multiple_of_eight_and_at_least_one() {
    let scale = |size| Font::new(Face::bitmap(), size).bitmap_scale();
    assert_eq!((scale(0), scale(3), scale(8), scale(11), scale(12), scale(16), scale(23)), (1, 1, 1, 1, 2, 2, 3));
    assert_eq!(Font::default().size, 16);
    assert!(!Font::default().face.is_outline());
  }

  #[test]
  fn files_that_are_not_fonts_give_no_face() {
    assert!(Face::from_bytes(b"not a font").is_none());
    assert!(Face::from_file("no/such/file.ttf").is_none());
  }

  #[test]
  fn faces_compare_by_identity_and_the_system_face_is_looked_up_once() {
    assert_eq!(Face::bitmap(), Face::bitmap());
    assert_eq!(Face::system(), Face::system());
    if Face::system().is_outline() {
      assert_ne!(Face::system(), Face::bitmap());
    }
  }
}
