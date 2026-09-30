use std::{borrow::Cow, sync::Arc};

use abra_core::{Area, Color, Fill, IntoNumber, Path};
use drawing::shader_from_fill_with_path;
use primitives::Image;
use swash::{
  GlyphId,
  scale::{Render, ScaleContext, Source},
};

use crate::font::Font;

const DEFAULT_DPI: f32 = 96.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
  #[default]
  Left,
  Center,
  Right,
  Justify,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextJustify {
  #[default]
  None,
  Left,
  Center,
  Right,
  InterWord,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WordWrap {
  #[default]
  None,
  Normal,
  BreakWord,
  Anywhere,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
  #[default]
  Clip,
  Ellipsis,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextStyle {
  #[default]
  Normal,
  Italic,
  Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDecoration {
  Underline,
  Strikethrough,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextSize {
  /// Text size specified in physical pixels.
  Pixels(f32),
  /// Automatically determines the text size based on the layout constraints.
  Auto,
  /// Text size specified as a percentage of the constrained layout area.
  Percent(f32),
  /// Text size specified in typographic points.
  Points(f32),
}

impl TextSize {
  /// Creates a text size in pixels per em.
  pub fn pixels(p_size: impl IntoNumber) -> Self {
    Self::Pixels(p_size.into())
  }

  /// Creates a text size in typographic points.
  pub fn points(p_size: impl IntoNumber) -> Self {
    Self::Points(p_size.into())
  }

  /// Creates a text size as a percentage of the constrained layout area.
  pub fn percent(p_size: impl IntoNumber) -> Self {
    Self::Percent(p_size.into())
  }
}

impl From<f32> for TextSize {
  fn from(p_size: f32) -> Self {
    Self::Pixels(p_size)
  }
}

abra_core::impl_into_number!(TextSize);

#[derive(Clone, Copy)]
struct ShapedGlyph {
  id: GlyphId,
  advance: i32,
  x_offset: i32,
  y_offset: i32,
  cluster: usize,
}

struct LayoutLine {
  text: String,
  paragraph_end: bool,
}

/// A piece of text paired with a font, ready to be rasterized.
///
/// Created via [`Font::text`] and configured with `with_*` builder methods.
/// Converts into an [`Image`] or `Arc<Image>`, so it can be passed anywhere an
/// image is expected, such as `Canvas::add_layer_from_image`.
#[derive(Clone)]
pub struct Text {
  font: Font,
  content: TextContent,
  appearance: TextAppearance,
  layout: TextLayout,
}

#[derive(Clone)]
pub struct TextContent {
  pub paragraphs: Vec<String>,
}

impl Default for TextContent {
  fn default() -> Self {
    Self { paragraphs: Vec::new() }
  }
}

#[derive(Clone)]
pub struct TextAppearance {
  pub size: TextSize,
  pub fill: Fill<'static>,
  pub weight: u32,
  pub style: TextStyle,
  pub variations: Vec<(String, f32)>,
  pub open_type_features: Vec<String>,
  pub decorations: Vec<TextDecoration>,
}

impl Default for TextAppearance {
  fn default() -> Self {
    Self {
      size: TextSize::points(16),
      fill: Fill::Solid(Color::black()),
      weight: 400,
      style: TextStyle::Normal,
      variations: Vec::new(),
      open_type_features: Vec::new(),
      decorations: Vec::new(),
    }
  }
}

#[derive(Clone)]
pub struct TextLayout {
  pub line_height: Option<f32>,
  pub paragraph_spacing: Option<f32>,
  pub letter_spacing: f32,
  pub word_spacing: f32,
  pub word_wrap: Option<WordWrap>,
  pub width: Option<f32>,
  pub height: Option<f32>,
  pub dpi: f32,
  pub max_lines: Option<usize>,
  pub overflow: TextOverflow,
  pub align: Option<TextAlign>,
  pub justify: Option<TextJustify>,
}

impl Default for TextLayout {
  fn default() -> Self {
    Self {
      line_height: None,
      paragraph_spacing: None,
      letter_spacing: 0.0,
      word_spacing: 0.0,
      word_wrap: Some(WordWrap::None),
      width: None,
      height: None,
      dpi: DEFAULT_DPI,
      max_lines: None,
      overflow: TextOverflow::Clip,
      align: Some(TextAlign::Left),
      justify: Some(TextJustify::None),
    }
  }
}

impl Text {
  /// Creates a new `Text` builder for `p_text` with default size (32px) and color (black).
  pub fn new(p_font: &Font, p_text: impl Into<String>) -> Self {
    Text {
      font: p_font.clone(),
      content: TextContent {
        paragraphs: vec![p_text.into()],
      },
      appearance: TextAppearance::default(),
      layout: TextLayout::default(),
    }
  }

  /// Replaces the text content configuration.
  pub fn with_content(mut self, p_content: TextContent) -> Self {
    self.content = p_content;
    self
  }

  /// Applies an appearance configuration through the granular appearance setters.
  pub fn with_appearance(mut self, p_appearance: TextAppearance) -> Self {
    self = self
      .with_size(p_appearance.size)
      .with_fill(p_appearance.fill)
      .with_weight(p_appearance.weight)
      .with_style(p_appearance.style);
    for (axis, value) in p_appearance.variations {
      self = self.with_variation(axis, value);
    }
    for feature in p_appearance.open_type_features {
      self = self.with_open_type_feature(feature);
    }
    for decoration in p_appearance.decorations {
      self = self.with_decoration(decoration);
    }
    self
  }

  /// Applies a layout configuration through the granular layout setters.
  pub fn with_layout(mut self, p_layout: TextLayout) -> Self {
    if let Some(line_height) = p_layout.line_height {
      self = self.with_line_height(line_height);
    }
    if let Some(paragraph_spacing) = p_layout.paragraph_spacing {
      self = self.with_paragraph_spacing(paragraph_spacing);
    }
    self =
      self.with_letter_spacing(p_layout.letter_spacing).with_word_spacing(p_layout.word_spacing).with_dpi(p_layout.dpi);
    if let Some(word_wrap) = p_layout.word_wrap {
      self = self.with_word_wrap(word_wrap);
    }
    if let Some(width) = p_layout.width {
      self = self.with_width(width);
    }
    if let Some(height) = p_layout.height {
      self = self.with_height(height);
    }
    if let Some(max_lines) = p_layout.max_lines {
      self = self.with_max_lines(max_lines);
    }
    if let Some(align) = p_layout.align {
      self = self.with_alignment(align);
    }
    if let Some(justify) = p_layout.justify {
      self = self.with_justification(justify);
    }
    self.with_overflow(p_layout.overflow)
  }

  /// Appends a paragraph. Each chained call is separated by [`with_paragraph_spacing`].
  pub fn text(mut self, p_text: impl Into<String>) -> Self {
    self.content.paragraphs.push(p_text.into());
    self
  }

  /// Sets the size of the text.
  /// The size can be specified in pixels, points, as a percentage of the layout area,
  /// or automatically determined based on layout constraints.
  pub fn with_size(mut self, p_size: impl Into<TextSize>) -> Self {
    self.appearance.size = p_size.into();
    self
  }

  /// Sets the fill (color, gradient, or image) used to fill the rasterized text.
  pub fn with_fill<'a>(mut self, p_color: impl Into<Fill<'a>>) -> Self {
    self.appearance.fill = match p_color.into() {
      Fill::Solid(color) => Fill::Solid(color),
      Fill::Gradient(gradient) => Fill::Gradient(Cow::Owned(gradient.into_owned())),
      Fill::Image(image) => Fill::Image(image),
    };
    self
  }

  /// Sets the weight of the text (100-900, where 400 is regular and 700 is bold).
  /// For variable fonts, this selects the `wght` axis instance to render.
  pub fn with_weight(mut self, p_weight: u32) -> Self {
    self.appearance.weight = p_weight;
    self
  }

  /// Selects the italic or oblique variation axis when the font supports it.
  pub fn with_style(mut self, p_style: TextStyle) -> Self {
    self.appearance.style = p_style;
    self
  }

  /// Sets a four-character OpenType variation axis such as `"wdth"` or `"opsz"`.
  pub fn with_variation(mut self, p_axis: impl Into<String>, p_value: impl IntoNumber) -> Self {
    let axis = p_axis.into();
    let value: f32 = p_value.into();
    if let Some((_, existing_value)) =
      self.appearance.variations.iter_mut().find(|(existing_axis, _)| *existing_axis == axis)
    {
      *existing_value = value;
    } else {
      self.appearance.variations.push((axis, value));
    }
    self
  }

  /// Enables or disables an OpenType shaping feature, such as `"liga=0"` or `"tnum=1"`.
  /// Invalid feature strings are ignored during rendering.
  pub fn with_open_type_feature(mut self, p_feature: impl Into<String>) -> Self {
    self.appearance.open_type_features.push(p_feature.into());
    self
  }

  /// Adds an underline or strikethrough to every rendered line.
  pub fn with_decoration(mut self, p_decoration: TextDecoration) -> Self {
    if !self.appearance.decorations.contains(&p_decoration) {
      self.appearance.decorations.push(p_decoration);
    }
    self
  }

  /// Sets the line height in pixels used to space consecutive lines of text.
  /// Defaults to the font's recommended line height (ascent + descent + line gap) when unset.
  pub fn with_line_height(mut self, p_line_height: impl IntoNumber) -> Self {
    self.layout.line_height = Some(p_line_height.into());
    self
  }

  /// Sets extra spacing between paragraphs added with [`text`](Self::text).
  /// Defaults to half the resolved line height when unset.
  pub fn with_paragraph_spacing(mut self, p_paragraph_spacing: impl IntoNumber) -> Self {
    self.layout.paragraph_spacing = Some(p_paragraph_spacing.into());
    self
  }

  /// Sets the letter spacing in pixels used to space consecutive characters of text.
  pub fn with_letter_spacing(mut self, p_letter_spacing: impl IntoNumber) -> Self {
    self.layout.letter_spacing = p_letter_spacing.into();
    self
  }

  /// Sets the word spacing in pixels used to space consecutive words of text.
  pub fn with_word_spacing(mut self, p_word_spacing: impl IntoNumber) -> Self {
    self.layout.word_spacing = p_word_spacing.into();
    self
  }

  /// Sets how text wraps when a width is supplied with [`with_width`].
  pub fn with_word_wrap(mut self, p_word_wrap: WordWrap) -> Self {
    self.layout.word_wrap = Some(p_word_wrap);
    self
  }

  /// Sets the layout width used for wrapping, alignment, and justification.
  /// When unset, the natural longest line supplies the layout width.
  pub fn with_width(mut self, p_width: impl IntoNumber) -> Self {
    self.layout.width = Some(p_width.into());
    self
  }

  /// Sets the maximum layout height used for automatic sizing and overflow.
  pub fn with_height(mut self, p_height: impl IntoNumber) -> Self {
    self.layout.height = Some(p_height.into());
    self
  }

  /// Sets the resolution used to convert [`TextSize::Points`] to pixels.
  /// Defaults to 96 DPI.
  pub fn with_dpi(mut self, p_dpi: impl IntoNumber) -> Self {
    self.layout.dpi = p_dpi.into();
    self
  }

  /// Limits the number of rendered lines. Use [`with_overflow`] to control truncation.
  pub fn with_max_lines(mut self, p_max_lines: usize) -> Self {
    self.layout.max_lines = Some(p_max_lines);
    self
  }

  /// Sets how text exceeding [`with_max_lines`] is represented.
  pub fn with_overflow(mut self, p_overflow: TextOverflow) -> Self {
    self.layout.overflow = p_overflow;
    self
  }

  /// Sets the horizontal alignment of each line within the text's overall width.
  pub fn with_alignment(mut self, p_align: TextAlign) -> Self {
    self.layout.align = Some(p_align);
    self
  }

  /// Sets the justification mode for lines that are shorter than the longest line.
  pub fn with_justification(mut self, p_justify: TextJustify) -> Self {
    self.layout.justify = Some(p_justify);
    self
  }
}

impl From<Text> for Image {
  fn from(p_text: Text) -> Image {
    p_text.font.render_text(&p_text)
  }
}

impl From<Text> for Arc<Image> {
  fn from(p_text: Text) -> Arc<Image> {
    Arc::new(p_text.into())
  }
}

impl Font {
  /// Creates a renderable [`Text`] builder for `p_text` using this font. Configure it with
  /// `with_size`/`with_fill`/`with_weight`, then convert it into an `Image` or pass it
  /// directly to APIs that accept `Into<Arc<Image>>`, such as `Canvas::add_layer_from_image`.
  pub fn text(&self, p_text: impl Into<String>) -> Text {
    Text::new(self, p_text)
  }

  /// Rasterizes `p_text` into a new transparent image tinted with its color, sized tightly
  /// around the glyphs. Lines are separated by `\n` and spaced using `p_text`'s line height,
  /// and characters/spaces are spaced using its letter/word spacing.
  /// Honors `p_text`'s weight via the font's `wght` variation axis, if it has one.
  pub fn render_text(&self, p_text: &Text) -> Image {
    let font_ref = self.as_ref();
    let mut variations = p_text.appearance.variations.clone();
    let mut set_variation = |axis: &str, value: f32| {
      if let Some((_, existing_value)) = variations.iter_mut().find(|(existing_axis, _)| existing_axis == axis) {
        *existing_value = value;
      } else {
        variations.push((axis.to_owned(), value));
      }
    };
    set_variation("wght", p_text.appearance.weight as f32);
    match p_text.appearance.style {
      TextStyle::Normal => {}
      TextStyle::Italic => set_variation("ital", 1.0),
      TextStyle::Oblique => set_variation("slnt", -12.0),
    }

    let shaping_face = rustybuzz::Face::from_slice(self.bytes(), 0).expect("loaded font must be valid for shaping");
    let units_per_em = shaping_face.units_per_em() as f32;
    let shape_features = p_text
      .appearance
      .open_type_features
      .iter()
      .filter_map(|feature| feature.parse::<rustybuzz::Feature>().ok())
      .collect::<Vec<_>>();
    let resolve_size = || match p_text.appearance.size {
      TextSize::Pixels(size) => size,
      TextSize::Auto => {
        let width_size = p_text.layout.width.filter(|width| *width > 0.0).map(|width| {
          let mut widest_size = 0.0f32;
          for paragraph in &p_text.content.paragraphs {
            for line in paragraph.split('\n') {
              let mut buffer = rustybuzz::UnicodeBuffer::new();
              buffer.push_str(line);
              let shaped = rustybuzz::shape(&shaping_face, &shape_features, buffer);
              let fixed_spacing = shaped.glyph_infos().iter().fold(0.0, |spacing, glyph| {
                let is_space = line
                  .get(glyph.cluster as usize..)
                  .and_then(|cluster| cluster.chars().next())
                  .is_some_and(|ch| ch == ' ');
                spacing + p_text.layout.letter_spacing + if is_space { p_text.layout.word_spacing } else { 0.0 }
              });
              let advances = shaped.glyph_positions().iter().map(|position| position.x_advance as f32).sum::<f32>();
              if advances > 0.0 {
                widest_size = widest_size.max(((width - fixed_spacing).max(1.0) * units_per_em / advances).max(1.0));
              }
            }
          }
          widest_size
        });
        let height_size = p_text.layout.height.filter(|height| *height > 0.0).map(|height| {
          let line_count =
            p_text.content.paragraphs.iter().map(|paragraph| paragraph.split('\n').count()).sum::<usize>();
          let paragraph_count = p_text.content.paragraphs.len().saturating_sub(1);
          let vertical_units = 1.0 + line_count.saturating_sub(1) as f32 * 0.9 + paragraph_count as f32 * 0.6;
          (height / vertical_units).max(1.0)
        });
        match (width_size.filter(|size| *size > 0.0), height_size) {
          (Some(width), Some(height)) => width.min(height),
          (Some(width), None) => width,
          (None, Some(height)) => height,
          (None, None) => 32.0,
        }
      }
      TextSize::Percent(percent) => p_text
        .layout
        .width
        .into_iter()
        .chain(p_text.layout.height)
        .reduce(f32::min)
        .map(|available_size| (available_size * percent / 100.0).max(1.0))
        .unwrap_or(32.0),
      TextSize::Points(points) => (points * p_text.layout.dpi / 72.0).max(1.0),
    };
    let size_px = resolve_size();
    let shaping_scale = size_px / units_per_em;

    let mut context = ScaleContext::new();
    let mut scaler = context
      .builder(font_ref)
      .size(size_px)
      .hint(true)
      .variations(variations.iter().map(|(axis, value)| (axis.as_str(), *value)))
      .build();

    let metrics = font_ref.metrics(&[]).scale(size_px);
    let line_height = p_text.layout.line_height.unwrap_or(size_px * 0.9);
    let paragraph_spacing = p_text.layout.paragraph_spacing.unwrap_or(size_px * 0.6);

    let shape_line = |line: &str| {
      let mut buffer = rustybuzz::UnicodeBuffer::new();
      buffer.push_str(line);
      let shaped = rustybuzz::shape(&shaping_face, &shape_features, buffer);
      shaped
        .glyph_infos()
        .iter()
        .zip(shaped.glyph_positions())
        .map(|(info, position)| ShapedGlyph {
          id: info.glyph_id as GlyphId,
          advance: (position.x_advance as f32 * shaping_scale).round() as i32,
          x_offset: (position.x_offset as f32 * shaping_scale).round() as i32,
          y_offset: (position.y_offset as f32 * shaping_scale).round() as i32,
          cluster: info.cluster as usize,
        })
        .collect::<Vec<_>>()
    };
    let measure_line = |line: &str| {
      shape_line(line).iter().fold(0i32, |width, glyph| {
        let is_space = line.get(glyph.cluster..).and_then(|cluster| cluster.chars().next()).is_some_and(|ch| ch == ' ');
        width
          + glyph.advance
          + p_text.layout.letter_spacing.round() as i32
          + if is_space { p_text.layout.word_spacing.round() as i32 } else { 0 }
      })
    };

    let wrap_width = p_text.layout.width.map(|width| width.round() as i32).filter(|width| *width > 0);
    let word_wrap = p_text.layout.word_wrap.unwrap_or_default();
    let mut lines = Vec::new();
    for (paragraph_index, paragraph) in p_text.content.paragraphs.iter().enumerate() {
      for source_line in paragraph.split('\n') {
        let Some(wrap_width) = wrap_width else {
          lines.push(LayoutLine {
            text: source_line.to_owned(),
            paragraph_end: false,
          });
          continue;
        };

        match word_wrap {
          WordWrap::None => lines.push(LayoutLine {
            text: source_line.to_owned(),
            paragraph_end: false,
          }),
          WordWrap::Anywhere => {
            let mut line = String::new();
            for ch in source_line.chars() {
              let mut candidate = line.clone();
              candidate.push(ch);
              if !line.is_empty() && measure_line(&candidate) > wrap_width {
                lines.push(LayoutLine {
                  text: line,
                  paragraph_end: false,
                });
                line = ch.to_string();
              } else {
                line = candidate;
              }
            }
            lines.push(LayoutLine {
              text: line,
              paragraph_end: false,
            });
          }
          WordWrap::Normal | WordWrap::BreakWord => {
            let words: Vec<&str> = source_line.split_whitespace().collect();
            if words.is_empty() {
              lines.push(LayoutLine {
                text: String::new(),
                paragraph_end: false,
              });
              continue;
            }

            let mut line = String::new();
            for word in words {
              let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
              if measure_line(&candidate) <= wrap_width {
                line = candidate;
                continue;
              }

              if !line.is_empty() {
                lines.push(LayoutLine {
                  text: line,
                  paragraph_end: false,
                });
                line = String::new();
              }

              if word_wrap == WordWrap::Normal || measure_line(word) <= wrap_width {
                line = word.to_owned();
                continue;
              }

              for ch in word.chars() {
                let mut candidate = line.clone();
                candidate.push(ch);
                if !line.is_empty() && measure_line(&candidate) > wrap_width {
                  lines.push(LayoutLine {
                    text: line,
                    paragraph_end: false,
                  });
                  line = ch.to_string();
                } else {
                  line = candidate;
                }
              }
            }
            lines.push(LayoutLine {
              text: line,
              paragraph_end: false,
            });
          }
        }
      }
      if let Some(last_line) = lines.last_mut() {
        last_line.paragraph_end = paragraph_index + 1 < p_text.content.paragraphs.len();
      }
    }

    let height_max_lines = p_text.layout.height.filter(|height| *height > 0.0).map(|height| {
      let mut used_height = 0.0;
      let mut max_lines = 0;
      for line in &lines {
        let next_height = used_height + line_height;
        if next_height > height && max_lines > 0 {
          break;
        }
        used_height = next_height;
        max_lines += 1;
        if line.paragraph_end {
          used_height += paragraph_spacing;
        }
      }
      max_lines.max(1)
    });
    let max_lines = match (p_text.layout.max_lines, height_max_lines) {
      (Some(line_limit), Some(height_limit)) => Some(line_limit.min(height_limit)),
      (Some(line_limit), None) => Some(line_limit),
      (None, Some(height_limit)) => Some(height_limit),
      (None, None) => None,
    };
    let truncated = max_lines.is_some_and(|line_limit| lines.len() > line_limit);
    if let Some(max_lines) = max_lines {
      lines.truncate(max_lines);
    }
    if truncated && p_text.layout.overflow == TextOverflow::Ellipsis && !lines.is_empty() {
      let last_line = &mut lines.last_mut().unwrap().text;
      if let Some(width) = wrap_width {
        while !last_line.is_empty() && measure_line(&format!("{last_line}…")) > width {
          last_line.pop();
        }
      }
      last_line.push('…');
    }

    let mut line_widths = Vec::with_capacity(lines.len());
    let mut max_line_width = 0i32;
    for line in &lines {
      let line_width = measure_line(&line.text);
      line_widths.push(line_width);
      max_line_width = max_line_width.max(line_width);
    }
    let shaped_lines: Vec<Vec<ShapedGlyph>> = lines.iter().map(|line| shape_line(&line.text)).collect();

    let mut placements = Vec::new();
    let mut min_x = 0i32;
    let mut min_y = 0i32;
    let mut max_x = 0i32;
    let mut max_y = 0i32;
    let mut decoration_rects = Vec::new();
    let target_width =
      p_text.layout.width.map(|width| width.round() as i32).filter(|width| *width > 0).unwrap_or(max_line_width);
    let mut baseline = metrics.ascent.round() as i32;
    for (line_index, line) in lines.iter().enumerate() {
      let line_text = &line.text;
      let line_width = line_widths[line_index];
      let align_mode = p_text.layout.align.unwrap_or(TextAlign::Left);
      let should_justify =
        matches!(p_text.layout.justify, Some(TextJustify::InterWord)) || align_mode == TextAlign::Justify;
      let target_line_width = target_width.max(line_width);
      let align_offset = match align_mode {
        TextAlign::Left => 0,
        TextAlign::Center => ((target_line_width - line_width) / 2).max(0),
        TextAlign::Right => (target_line_width - line_width).max(0),
        TextAlign::Justify => 0,
      };

      let inter_word_gap = if should_justify {
        let word_count = line_text.split_whitespace().count();
        if word_count > 1 && target_width > line_width {
          let gap_slots = (word_count - 1) as i32;
          let gap_budget = target_width - line_width;
          Some((gap_budget / gap_slots, gap_budget % gap_slots))
        } else {
          None
        }
      } else {
        None
      };

      let letter_gap = if should_justify
        && line_text.split_whitespace().count() <= 1
        && line_text.chars().count() > 1
        && target_width > line_width
      {
        let letter_slots = (line_text.chars().count() - 1) as i32;
        let gap_budget = target_width - line_width;
        Some((gap_budget / letter_slots, gap_budget % letter_slots))
      } else {
        None
      };

      let mut pen_x = align_offset;
      let mut space_index = 0usize;
      let glyphs = &shaped_lines[line_index];
      for (glyph_index, glyph) in glyphs.iter().enumerate() {
        if let Some(glyph_image) = Render::new(&[Source::Outline]).render(&mut scaler, glyph.id) {
          let placement = glyph_image.placement;
          if placement.width > 0 && placement.height > 0 {
            let x = pen_x + glyph.x_offset + placement.left;
            let y = baseline - glyph.y_offset - placement.top;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + placement.width as i32);
            max_y = max_y.max(y + placement.height as i32);
            placements.push((x, y, placement.width, placement.height, glyph_image.data));
          }
        }

        pen_x += glyph.advance;
        pen_x += p_text.layout.letter_spacing.round() as i32;
        let is_space =
          line.text.get(glyph.cluster..).and_then(|cluster| cluster.chars().next()).is_some_and(|ch| ch == ' ');

        if is_space {
          let extra_gap = inter_word_gap
            .map(|(base, remainder)| {
              let extra = if space_index < remainder as usize { 1 } else { 0 };
              space_index += 1;
              p_text.layout.word_spacing.round() as i32 + base + extra
            })
            .unwrap_or_else(|| p_text.layout.word_spacing.round() as i32);
          pen_x += extra_gap;
        } else if glyph_index + 1 < glyphs.len() && letter_gap.is_some() {
          let extra_gap = letter_gap
            .map(|(base, remainder)| {
              let extra = if glyph_index < remainder as usize { 1 } else { 0 };
              base + extra
            })
            .unwrap_or(0);
          pen_x += extra_gap;
        }
      }

      let decoration_width = (pen_x - align_offset).max(0) as u32;
      let decoration_thickness = (size_px * 0.05).round().max(1.0) as u32;
      for decoration in &p_text.appearance.decorations {
        let y = match decoration {
          TextDecoration::Underline => baseline - (metrics.descent * 0.35).round() as i32,
          TextDecoration::Strikethrough => baseline - (metrics.ascent * 0.3).round() as i32,
        };
        if decoration_width > 0 {
          min_x = min_x.min(align_offset);
          min_y = min_y.min(y);
          max_x = max_x.max(align_offset + decoration_width as i32);
          max_y = max_y.max(y + decoration_thickness as i32);
          decoration_rects.push((align_offset, y, decoration_width, decoration_thickness));
        }
      }

      baseline += line_height.round() as i32;
      if line.paragraph_end {
        baseline += paragraph_spacing.round() as i32;
      }
    }

    // Glyph bitmaps (e.g. tall ascenders on bold weights) can extend beyond the font's
    // nominal ascent/descent, so size the image from the actual rendered bounds instead.
    let offset_x = -min_x;
    let offset_y = -min_y;
    let width = (max_x + offset_x).max(1) as u32;
    let height = (max_y + offset_y).max(1) as u32;
    let mut image = Image::new(width, height);
    let fallback_path = Path::from(Area::rect((0, 0), (width as f32, height as f32)));
    let shader = shader_from_fill_with_path(p_text.appearance.fill.clone(), Some(fallback_path));

    for (x, y, glyph_width, glyph_height, bitmap) in placements {
      let x = x + offset_x;
      let y = y + offset_y;
      for gy in 0..glyph_height {
        for gx in 0..glyph_width {
          let coverage = bitmap[(gy * glyph_width + gx) as usize];
          if coverage == 0 {
            continue;
          }
          let px = x + gx as i32;
          let py = y + gy as i32;
          if px < 0 || py < 0 || px as u32 >= width || py as u32 >= height {
            continue;
          }
          let (red, green, blue, alpha) = shader.shade(px as f32 + 0.5, py as f32 + 0.5);
          let alpha = (u16::from(alpha) * u16::from(coverage) / 255) as u8;
          image.set_pixel(px as u32, py as u32, (red, green, blue, alpha));
        }
      }
    }

    for (x, y, decoration_width, decoration_height) in decoration_rects {
      let x = x + offset_x;
      let y = y + offset_y;
      for py in 0..decoration_height {
        for px in 0..decoration_width {
          let (red, green, blue, alpha) = shader.shade(x as f32 + px as f32 + 0.5, y as f32 + py as f32 + 0.5);
          image.set_pixel((x + px as i32) as u32, (y + py as i32) as u32, (red, green, blue, alpha));
        }
      }
    }

    image
  }
}
