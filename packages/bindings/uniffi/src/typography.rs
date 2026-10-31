//! `Font` and `Text` from `typography`: a font loaded from a file, and text set in it that becomes an image.
//!
//! What is not exported, and why:
//! - `Font::from_path` and `Font::from_bytes` return a `Result` in the library; here a file that is not a font panics, as the
//!   library's other bad input does.
//! - `Text::with_content`, `with_appearance` and `with_layout` take whole `TextContent`, `TextAppearance` and `TextLayout` structs
//!   and only call the granular setters below, so the setters are exported and the structs are not.
//! - `Text::with_fill` takes anything that becomes a `Fill<'static>`; here it takes a `Fill`.
//! - `FontLoader` and `load_fonts` find fonts on the system; a font here is loaded from a path or bytes.
//! - A number that is a `usize` in the library (`with_max_lines`) is a `u32` here.
//! - `From<Text> for Image` is `Text::to_image`.

use std::sync::Arc;

use abra::typography::prelude::{
  Font as library_font, Text as library_text, TextAlign, TextDecoration, TextJustify, TextOverflow, TextSize, TextStyle,
  WordWrap,
};

use crate::{Fill, Image};

/// How wide the lines of text are placed within the text's width.
#[uniffi::remote(Enum)]
pub enum TextAlign {
  Left,
  Center,
  Right,
  Justify,
}

/// How the lines shorter than the longest one are justified.
#[uniffi::remote(Enum)]
pub enum TextJustify {
  None,
  Left,
  Center,
  Right,
  InterWord,
}

/// How text wraps when a width is set.
#[uniffi::remote(Enum)]
pub enum WordWrap {
  None,
  Normal,
  BreakWord,
  Anywhere,
}

/// How text that is cut off by the maximum number of lines is shown.
#[uniffi::remote(Enum)]
pub enum TextOverflow {
  Clip,
  Ellipsis,
}

/// Which italic variation of the font is used.
#[uniffi::remote(Enum)]
pub enum TextStyle {
  Normal,
  Italic,
  Oblique,
}

/// A line drawn on every line of text.
#[uniffi::remote(Enum)]
pub enum TextDecoration {
  Underline,
  Strikethrough,
}

/// How big text is.
#[uniffi::remote(Enum)]
pub enum TextSize {
  /// Physical pixels per em.
  Pixels(f32),
  /// Worked out from the width and height of the text.
  Auto,
  /// A percentage of the text's width or height, whichever is smaller.
  Percent(f32),
  /// Typographic points, turned into pixels with the text's DPI.
  Points(f32),
}

/// A loaded font face.
#[derive(Clone, uniffi::Object)]
pub struct Font(pub library_font);

#[uniffi::export]
impl Font {
  /// Loads the font file (a `.ttf` or `.otf`) at this path. Panics if there is no such file or it is not a font.
  #[uniffi::constructor]
  pub fn from_path(path: String) -> Self {
    Self(library_font::from_path(&path).unwrap_or_else(|error| panic!("could not load the font {path}: {error}")))
  }

  /// Loads a font from the bytes of a `.ttf` or `.otf` file. Panics if they are not a font.
  #[uniffi::constructor]
  pub fn from_bytes(bytes: Vec<u8>) -> Self {
    Self(library_font::from_bytes(bytes).unwrap_or_else(|error| panic!("could not load the font: {error}")))
  }

  /// Text set in this font. Change it with the `with_` functions, then get it with [`Text::to_image`].
  pub fn text(&self, text: String) -> Arc<Text> {
    Arc::new(Text(self.0.text(text)))
  }
}

/// A piece of text set in a font. Each `with_` function gives a changed copy.
#[derive(Clone, uniffi::Object)]
pub struct Text(pub library_text);

#[uniffi::export]
impl Text {
  /// A copy of this text with another paragraph after it. Paragraphs are spaced by [`Text::with_paragraph_spacing`].
  pub fn text(&self, text: String) -> Arc<Text> {
    Arc::new(Text(self.0.clone().text(text)))
  }

  /// A copy of this text with this size.
  pub fn with_size(&self, size: TextSize) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_size(size)))
  }

  /// A copy of this text filled with a color, a gradient or an image.
  pub fn with_fill(&self, fill: Arc<Fill>) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_fill(fill.0.clone())))
  }

  /// A copy of this text with this weight, from 100 to 900, where 400 is regular and 700 is bold.
  pub fn with_weight(&self, weight: u32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_weight(weight)))
  }

  /// A copy of this text in this style.
  pub fn with_style(&self, style: TextStyle) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_style(style)))
  }

  /// A copy of this text with a four-letter OpenType variation axis, such as `wdth`, set to this value.
  pub fn with_variation(&self, axis: String, value: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_variation(axis, value)))
  }

  /// A copy of this text with an OpenType feature turned on or off, such as `liga=0` or `tnum=1`.
  pub fn with_open_type_feature(&self, feature: String) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_open_type_feature(feature)))
  }

  /// A copy of this text with a line drawn on every line of it.
  pub fn with_decoration(&self, decoration: TextDecoration) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_decoration(decoration)))
  }

  /// A copy of this text with this many pixels from one line to the next.
  pub fn with_line_height(&self, line_height: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_line_height(line_height)))
  }

  /// A copy of this text with this many pixels between paragraphs.
  pub fn with_paragraph_spacing(&self, paragraph_spacing: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_paragraph_spacing(paragraph_spacing)))
  }

  /// A copy of this text with this many pixels between letters.
  pub fn with_letter_spacing(&self, letter_spacing: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_letter_spacing(letter_spacing)))
  }

  /// A copy of this text with this many extra pixels between words.
  pub fn with_word_spacing(&self, word_spacing: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_word_spacing(word_spacing)))
  }

  /// A copy of this text that wraps this way when it has a width.
  pub fn with_word_wrap(&self, word_wrap: WordWrap) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_word_wrap(word_wrap)))
  }

  /// A copy of this text laid out in this many pixels of width, for wrapping, alignment and justification.
  pub fn with_width(&self, width: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_width(width)))
  }

  /// A copy of this text laid out in at most this many pixels of height, for automatic sizing and overflow.
  pub fn with_height(&self, height: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_height(height)))
  }

  /// A copy of this text with the DPI that points are turned into pixels with. It is 96 unless set.
  pub fn with_dpi(&self, dpi: f32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_dpi(dpi)))
  }

  /// A copy of this text showing at most this many lines. Set what happens to the rest with [`Text::with_overflow`].
  pub fn with_max_lines(&self, max_lines: u32) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_max_lines(max_lines as usize)))
  }

  /// A copy of this text that shows lines cut off by the maximum number of lines this way.
  pub fn with_overflow(&self, overflow: TextOverflow) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_overflow(overflow)))
  }

  /// A copy of this text with its lines placed this way within its width.
  pub fn with_alignment(&self, align: TextAlign) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_alignment(align)))
  }

  /// A copy of this text justified this way.
  pub fn with_justification(&self, justify: TextJustify) -> Arc<Text> {
    Arc::new(Text(self.0.clone().with_justification(justify)))
  }

  /// The text as a new transparent image, sized tightly around the letters.
  pub fn to_image(&self) -> Arc<Image> {
    Arc::new(Image::from_core(self.0.clone().into()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Color;

  /// A font from the system, or `None` where the machine has none of these.
  fn font() -> Option<Arc<Font>> {
    ["C:/Windows/Fonts/arial.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/System/Library/Fonts/Supplemental/Arial.ttf"]
      .into_iter()
      .find(|path| std::path::Path::new(path).is_file())
      .map(|path| Arc::new(Font::from_path(path.to_string())))
  }

  fn white() -> Arc<Fill> {
    Arc::new(Fill::solid(Arc::new(Color::from_rgba(255, 255, 255, 255))))
  }

  #[test]
  fn text_is_drawn_in_its_fill_and_clear_elsewhere() {
    let Some(font) = font() else { return };
    let image = font.text("Hi".to_string()).with_size(TextSize::Pixels(40.0)).with_fill(white()).to_image();
    let (width, height) = (image.width(), image.height());
    let pixels: Vec<Vec<u8>> = (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).map(|(x, y)| image.get_pixel(x, y).unwrap()).collect();
    assert!(pixels.iter().any(|pixel| pixel == &vec![255, 255, 255, 255]));
    assert!(pixels.iter().any(|pixel| pixel[3] == 0));
  }

  #[test]
  fn a_bigger_size_makes_a_bigger_image() {
    let Some(font) = font() else { return };
    let small = font.text("Hi".to_string()).with_size(TextSize::Pixels(20.0)).to_image();
    let big = font.text("Hi".to_string()).with_size(TextSize::Pixels(60.0)).to_image();
    assert!(big.width() > small.width() && big.height() > small.height());
  }

  #[test]
  fn another_paragraph_makes_the_image_taller_and_a_copy_leaves_the_first_alone() {
    let Some(font) = font() else { return };
    let one = font.text("Hi".to_string()).with_size(TextSize::Pixels(30.0));
    let two = one.text("There".to_string());
    assert!(two.to_image().height() > one.to_image().height());
    assert_eq!(one.to_image().height(), font.text("Hi".to_string()).with_size(TextSize::Pixels(30.0)).to_image().height());
  }

  #[test]
  fn a_width_wraps_the_text_onto_more_lines() {
    let Some(font) = font() else { return };
    let plain = font.text("one two three four".to_string()).with_size(TextSize::Pixels(30.0));
    let wrapped = plain.with_width(120.0).with_word_wrap(WordWrap::Normal);
    assert!(wrapped.to_image().height() > plain.to_image().height());
  }

  #[test]
  #[should_panic(expected = "could not load the font")]
  fn a_file_that_is_not_there_panics() {
    Font::from_path("no/such/font.ttf".to_string());
  }
}
