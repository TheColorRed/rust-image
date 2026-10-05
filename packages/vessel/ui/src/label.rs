use vessel_api::prelude::*;
use vessel_macros::Component;

use crate::{UiBackground, background::BackgroundLayer};

/// Vertical placement of a label's text inside its content box. This setting is not inherited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalAlign {
  /// Align to the top edge.
  Top,
  /// Center vertically (the default).
  #[default]
  Center,
  /// Align to the bottom edge.
  Bottom,
}

/// Text, drawn using inherited horizontal alignment and centered vertically by default.
/// Use `with_text_align` on the label or a parent and `with_vertical_align` on the label to change placement.
/// Its size comes from the font it
/// inherits: set one on the label, or on any parent, with `set_font_size` or `set_font_face`. Where nothing sets one, the host's
/// font is used: a desktop window gives the system's, and the built-in bitmap font is the last resort (plain ASCII).
///
/// ```
/// use vessel_api::prelude::*;
/// use vessel_ui::{Container, Label, VerticalAlign};
///
/// let caption = Label::new("Warm").with_vertical_align(VerticalAlign::Bottom);
/// let card = Container::new().with_text_align(TextAlign::Center).add(&caption);
/// caption.set_text_align(TextAlign::Right); // override the container
/// caption.inherit_text_align(); // use the container's center alignment again
/// ```
///
/// Like a container, a label accepts a theme/color fill or an image behind its text:
///
/// ```
/// use vessel_api::prelude::*;
/// use vessel_ui::{Fit, Image, Label};
///
/// let gradient = Image::new().with_fit(Fit::Stretch).with_pixels(1, 1, vec![0, 0, 0, 128]);
/// let caption = Label::new("Warm").with_background(&gradient);
/// caption.set_background(Background::None);
/// ```
#[derive(Clone, Component)]
#[component(plain)]
pub struct Label {
  component: Component,
  text: BehaviorSubject<String>,
  color: BehaviorSubject<Option<[u8; 4]>>,
  vertical_align: BehaviorSubject<VerticalAlign>,
  background: BackgroundLayer,
}

impl Label {
  component_builders!();

  /// Text in the theme's text color and the inherited font. Set `color` to use another.
  pub fn new(p_text: impl Into<String>) -> Self {
    let text = BehaviorSubject::new(p_text.into());
    let color = BehaviorSubject::new(None);
    let vertical_align = BehaviorSubject::new(VerticalAlign::default());

    let component = Component::new("label");
    let background = BackgroundLayer::new(&component);
    component.subject::<Canvas>().subscribe({
      let (text, color, vertical_align) = (text.clone(), color.clone(), vertical_align.clone());
      move |canvas| {
        let text = text.value();
        let (text_width, text_height) = canvas.text_size(&text);
        let color = color.value().unwrap_or(canvas.theme().text);
        let (left, top, width, height) = canvas.content_rect();
        let spare_x = width as i64 - text_width as i64;
        let spare_y = height as i64 - text_height as i64;
        let x = left as i64
          + match canvas.text_align() {
            TextAlign::Left => 0,
            TextAlign::Center => spare_x / 2,
            TextAlign::Right => spare_x,
          };
        let y = top as i64
          + match vertical_align.value() {
            VerticalAlign::Top => 0,
            VerticalAlign::Center => spare_y / 2,
            VerticalAlign::Bottom => spare_y,
          };
        canvas.draw_text(
          x.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
          y.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
          &text,
          color,
        );
      }
    });

    Self {
      component,
      text,
      color,
      vertical_align,
      background,
    }
  }

  /// Replaces the theme/color or image background behind the text. Use [`Background::None`] to clear it.
  /// Images follow their fit setting and are clipped to the label's rounded border box.
  /// An unreadable GPU background panics rather than silently disappearing.
  pub fn set_background(&self, p_background: impl Into<UiBackground>) {
    self.background.set(&self.component, p_background.into());
  }

  /// Sets a background and returns the label, preserving its text-specific builders.
  pub fn with_background(&self, p_background: impl Into<UiBackground>) -> Self {
    self.set_background(p_background);
    self.clone()
  }

  properties! {
    text: impl Into<String> => set_text, with_text;
    color: [u8; 4] => set_color, with_color;
    vertical_align: VerticalAlign => set_vertical_align, with_vertical_align;
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  fn lit(p_label: &Label) -> Vec<usize> {
    let mut component = p_label.component.clone();
    let frame = component.render(Duration::ZERO);
    frame.pixels.chunks_exact(4).enumerate().filter(|(_, pixel)| pixel[3] == 255).map(|(index, _)| index).collect()
  }

  #[test]
  fn it_draws_its_text_in_its_color_centered_up_and_down() {
    let label = Label::new("Hi").with_color([9, 8, 7, 255]);
    label.size().next((60, 40));
    let lit = lit(&label);
    assert!(!lit.is_empty());
    let rows: Vec<usize> = lit.iter().map(|index| index / 60).collect();
    // 16 pixel text in a 40 pixel room starts 12 pixels down and stays inside 12..28.
    assert!(rows.iter().all(|row| (12..28).contains(row)), "{:?}..{:?}", rows.first(), rows.last());
    assert!(lit.iter().all(|index| index % 60 < 32), "two letters are 32 pixels wide, starting at the left");
  }

  #[test]
  fn changing_its_text_redraws_it_and_its_size_follows_the_inherited_font() {
    let label = Label::new("A");
    label.size().next((100, 40));
    let before = lit(&label).len();
    assert!(!label.component.clone().has_changed());
    label.set_text("AAA");
    assert!(label.component.clone().has_changed());
    assert!(lit(&label).len() > before * 2);
    label.set_font_size(8);
    let small = lit(&label).len();
    label.set_font_size(16);
    assert_eq!(lit(&label).len(), small * 4);

    // A parent's font reaches it when the label has none of its own.
    label.inherit_font();
    let parent = Component::new("parent").with_font_size(8);
    parent.add(label.clone());
    assert_eq!(lit(&label).len(), small);
  }

  #[test]
  fn percentage_font_sizes_redraw_labels_when_the_parent_font_changes() {
    let label = Label::new("A").with_font_size(Units::Percent(50.0));
    let parent = Component::new("parent").with_font_size(16);
    parent.add(&label);
    label.size().next((100, 40));
    let small = lit(&label).len();
    assert_eq!(label.font().size, 8);
    assert!(!label.component.clone().has_changed());
    parent.set_font_size(32);
    assert!(label.component.clone().has_changed());
    assert_eq!(label.font().size, 16);
    assert_eq!(lit(&label).len(), small * 4);
  }

  #[test]
  fn its_color_follows_the_theme_unless_set() {
    let label = Label::new("Hi");
    label.size().next((40, 20));
    let color_of_first_lit_pixel = |label: &Label| {
      let mut component = label.component.clone();
      let frame = component.render(Duration::ZERO);
      let at = frame.pixels.chunks_exact(4).position(|pixel| pixel[3] == 255).unwrap() * 4;
      { let slice = &frame.pixels.as_slice()[at..at + 4]; let pixel: [u8; 4] = slice.try_into().unwrap(); pixel }
    };
    assert_eq!(color_of_first_lit_pixel(&label), Theme::light().text);
    label.set_default_theme(Theme::dark());
    assert_eq!(color_of_first_lit_pixel(&label), Theme::dark().text, "the host's dark theme reaches it");
    label.set_color([1, 2, 3, 255]);
    assert_eq!(color_of_first_lit_pixel(&label), [1, 2, 3, 255]);
  }

  #[test]
  fn it_has_the_box_settings_of_any_component_so_padding_moves_its_text() {
    let label = Label::new("I").with_radius(0);
    label.size().next((60, 40));
    let first_column = |label: &Label| lit(label).iter().map(|index| index % 60).min().unwrap();
    let before = first_column(&label);
    label.set_padding([0, 10]); // up and down 0, left and right 10
    assert_eq!(first_column(&label), before + 10, "the text starts 10 pixels further in");
    label.set_background(Background::Color([9, 9, 9, 255]));
    let mut component = label.component.clone();
    let frame = component.render(Duration::ZERO);
    assert_eq!(frame.pixels[..4], [9, 9, 9, 255], "and it can have a background like any component");
  }

  #[test]
  fn text_alignment_positions_glyphs_inside_padding_and_border() {
    for (horizontal, dx) in [(TextAlign::Left, 0), (TextAlign::Center, 10), (TextAlign::Right, 20)] {
      for (vertical, dy) in [
        (VerticalAlign::Top, 0),
        (VerticalAlign::Center, 8),
        (VerticalAlign::Bottom, 16),
      ] {
        let label = Label::new("I")
          .with_font_size(8)
          .with_padding([2, 4, 6, 8])
          .with_border_width(1)
          .with_border_color([0; 4])
          .with_text_align(horizontal)
          .with_vertical_align(vertical);
        label.size().next((42, 34));
        let actual = lit(&label);
        let baseline = Label::new("I").with_font_size(8);
        baseline.size().next((8, 8));
        let expected: Vec<usize> =
          lit(&baseline).iter().map(|index| (index / 8 + 3 + dy) * 42 + index % 8 + 9 + dx).collect();
        assert_eq!(actual, expected, "{horizontal:?}, {vertical:?}");
      }
    }
  }

  #[test]
  fn horizontal_alignment_inherits_and_overrides_redraw_reactively() {
    let parent = Component::new("parent").with_text_align(TextAlign::Center);
    let middle = Component::new("middle");
    let label = Label::new("I").with_font_size(8);
    parent.add(&middle);
    middle.add(&label);
    label.size().next((40, 24));
    let centered = lit(&label);
    assert_eq!(label.text_align(), TextAlign::Center);
    assert!(!label.component.clone().has_changed());
    parent.set_text_align(TextAlign::Right);
    assert!(label.component.clone().has_changed());
    assert_eq!(label.text_align(), TextAlign::Right);
    assert_eq!(lit(&label), centered.iter().map(|index| index + 16).collect::<Vec<_>>());
    label.set_text_align(TextAlign::Left);
    parent.set_text_align(TextAlign::Center);
    assert_eq!(label.text_align(), TextAlign::Left);
    label.inherit_text_align();
    assert_eq!(lit(&label), centered);
    label.set_vertical_align(VerticalAlign::Bottom);
    assert!(label.component.clone().has_changed());
    assert_eq!(lit(&label), centered.iter().map(|index| index + 8 * 40).collect::<Vec<_>>());
    parent.inherit_text_align();
    assert_eq!(label.text_align(), TextAlign::Left);
  }

  #[test]
  fn oversized_and_empty_text_remain_safe_for_all_alignments() {
    for horizontal in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
      for vertical in [VerticalAlign::Top, VerticalAlign::Center, VerticalAlign::Bottom] {
        let label =
          Label::new("Oversized").with_font_size(16).with_text_align(horizontal).with_vertical_align(vertical);
        label.size().next((10, 10));
        lit(&label);
        label.set_text("");
        assert!(lit(&label).is_empty());
      }
    }
  }

  #[test]
  fn image_backgrounds_draw_before_text_follow_updates_and_can_be_cleared() {
    use crate::{Fit, Image};

    let image = Image::new().with_fit(Fit::Stretch).with_pixels(1, 1, vec![200, 0, 0, 255]);
    let label = Label::new("I")
      .with_background(&image)
      .with_text("OK")
      .with_color([0, 255, 0, 255])
      .with_radius(0)
      .with_font_size(8);
    label.size().next((40, 24));
    let render = || label.component.clone().render(Duration::ZERO);
    let frame = render();
    assert_eq!(&frame.pixels.as_slice()[..4], &[200, 0, 0, 255]);
    assert!(frame.pixels.chunks_exact(4).any(|pixel| pixel == [0, 255, 0, 255]));
    image.set_pixels(1, 1, vec![0, 0, 200, 255]);
    assert!(label.component.clone().has_changed());
    assert_eq!(&render().pixels[..4], &[0, 0, 200, 255]);
    label.size().next((80, 48));
    assert_eq!(&render().pixels[..4], &[0, 0, 200, 255]);
    assert_eq!(image.size().value(), (80, 48));
    label.set_background(Background::None);
    assert_eq!(&render().pixels[..4], &[0; 4]);
    image.set_pixels(1, 1, vec![100, 0, 0, 255]);
    assert!(!label.component.clone().has_changed(), "clearing forgets the old image dependencies");
    label.set_background(Background::Surface);
    assert_eq!(&render().pixels[..4], &Theme::light().surface);
    label.set_background(image);
    assert_eq!(&render().pixels[..4], &[100, 0, 0, 255]);
  }

  #[test]
  fn translucent_label_backgrounds_blend_with_the_parent_and_keep_rounded_borders() {
    use crate::Image;

    let image = Image::new().with_pixels(1, 1, vec![0, 0, 0, 128]);
    let label =
      Label::new("").with_background(image).with_radius(8).with_border_width(2).with_border_color([0, 255, 0, 255]);
    let parent = Component::new("parent")
      .with_size(40, 24)
      .with_radius(0)
      .with_background(Background::Color([200, 0, 0, 255]))
      .add(&label);
    let frame = parent.clone().render(Duration::ZERO);
    let pixel = |x: usize, y: usize| &frame.pixels.as_slice()[(y * 40 + x) * 4..(y * 40 + x) * 4 + 4];
    assert_eq!(pixel(0, 0), &[200, 0, 0, 255]);
    assert_eq!(pixel(20, 12), &[99, 0, 0, 255]);
    assert_eq!(pixel(20, 0), &[0, 255, 0, 255]);
    assert_eq!(    pixel(3, 2), &[0, 255, 0, 255], "the border is above the image at the curved corner");
  }

  #[test]
  fn empty_text_draws_nothing() {
    let label = Label::new("");
    label.size().next((10, 10));
    assert!(lit(&label).is_empty());
  }
}
