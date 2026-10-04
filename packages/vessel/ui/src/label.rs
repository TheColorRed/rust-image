
use vessel_api::prelude::*;
use vessel_macros::Component;

/// Text, drawn at the left and vertically centered in the room its parent gives it. Its size comes from the font it
/// inherits: set one on the label, or on any parent, with `set_font_size` or `set_font_face`. Where nothing sets one, the host's
/// font is used: a desktop window gives the system's, and the built-in bitmap font is the last resort (plain ASCII).
#[derive(Clone, Component)]
#[component(plain)]
pub struct Label {
  component: Component,
  text: BehaviorSubject<String>,
  color: BehaviorSubject<Option<[u8; 4]>>,
}

impl Label {
  /// Text in the theme's text color and the inherited font. Set `color` to use another.
  pub fn new(p_text: impl Into<String>) -> Self {
    let text = BehaviorSubject::new(p_text.into());
    let color = BehaviorSubject::new(None);

    let component = Component::new("label");
    component.subject::<Canvas>().subscribe({
      let (text, color) = (text.clone(), color.clone());
      move |canvas| {
        let text = text.value();
        let (_, text_height) = canvas.text_size(&text);
        let color = color.value().unwrap_or(canvas.theme().text);
        let (left, top, _, height) = canvas.content_rect();
        canvas.draw_text(left, top + (height as i32 - text_height as i32) / 2, &text, color);
      }
    });

    Self { component, text, color }
  }

  properties! {
    text: impl Into<String> => set_text, with_text;
    color: [u8; 4] => set_color, with_color;
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
  fn its_color_follows_the_theme_unless_set() {
    let label = Label::new("Hi");
    label.size().next((40, 20));
    let color_of_first_lit_pixel = |label: &Label| {
      let mut component = label.component.clone();
      let frame = component.render(Duration::ZERO);
      let at = frame.pixels.chunks_exact(4).position(|pixel| pixel[3] == 255).unwrap() * 4;
      <[u8; 4]>::try_from(&frame.pixels[at..at + 4]).unwrap()
    };
    assert_eq!(color_of_first_lit_pixel(&label), Theme::light().text);
    label.set_default_theme(Theme::dark());
    assert_eq!(color_of_first_lit_pixel(&label), Theme::dark().text, "the host's dark theme reaches it");
    label.set_color([1, 2, 3, 255]);
    assert_eq!(color_of_first_lit_pixel(&label), [1, 2, 3, 255]);
  }

  #[test]
  fn it_has_the_box_settings_of_any_component_so_padding_moves_its_text() {
    let label = Label::new("I");
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
  fn empty_text_draws_nothing() {
    let label = Label::new("");
    label.size().next((10, 10));
    assert!(lit(&label).is_empty());
  }
}
