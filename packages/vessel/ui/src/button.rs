use std::sync::{Arc, Mutex};

use vessel_api::prelude::*;
use vessel_macros::Component;

/// Sent on a [`Button`] when it is pressed and let go over itself, or activated with Enter or Space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clicked;

/// A button with a label. By default it is a plain control like the system's: a light gray face, a thin border and ordinary
/// text. `with_primary(true)` makes it the accent-colored call to action instead. It darkens while pressed and sends
/// [`Clicked`] when it is let go over itself.
///
/// It is a component, so give it a size with `width` and `height` (or let it fill), add it to a parent, and listen with
/// `button.subscribe(|_: &Clicked| ..)`. Use several; each has its own state.
#[derive(Clone, Component)]
#[component(plain)]
pub struct Button {
  component: Component,
  label: BehaviorSubject<String>,
  text_color: BehaviorSubject<Option<[u8; 4]>>,
  primary: BehaviorSubject<bool>,
}

impl Button {
  /// A plain button with the theme's control look: its face as the background, a 1 pixel border, rounded corners, and the
  /// control text color. Like any component it has the box settings (`set_background`, `set_radius`, `set_padding`,
  /// `set_margin`, `set_border_width`, ...), and they win over the theme for this button. `primary` gives the accent look.
  pub fn new(p_label: impl Into<String>) -> Self {
    let label = BehaviorSubject::new(p_label.into());
    let text_color = BehaviorSubject::new(None);
    let primary = BehaviorSubject::new(false);
    let pressed = BehaviorSubject::new(false);

    let component =
      Component::new("button").with_background(Background::Control).with_border_width(1).with_radius(Radius::Theme);
    component.subject::<Canvas>().subscribe({
      let (label, text_color, primary, pressed) = (label.clone(), text_color.clone(), primary.clone(), pressed.clone());
      move |canvas| {
        let theme = canvas.theme();
        // Pressed looks a little darker, following the button's own shape.
        if pressed.value() {
          let (width, height) = (canvas.width(), canvas.height());
          canvas.fill_rounded_rect(0, 0, width, height, canvas.radius(), [0, 0, 0, 38]);
        }
        let label = label.value();
        let (text_width, text_height) = canvas.text_size(&label);
        let (left, top, width, height) = canvas.content_rect();
        canvas.draw_text(
          left + (width as i32 - text_width as i32) / 2,
          top + (height as i32 - text_height as i32) / 2,
          &label,
          text_color.value().unwrap_or(if primary.value() { theme.accent_text } else { theme.control_text }),
        );
      }
    });

    // Like a browser's button, it does not take its parents' font; call `set_inherits_font(true)` to make it.
    component.set_inherits_font(false);

    // Press and let go over the button is a click. The pointer stays with the button while it is held, so where it was let
    // go decides.
    let at = Arc::new(Mutex::new((0.0f32, 0.0f32)));
    let this = component.downgrade();
    component.subject::<PointerEvent>().subscribe({
      let pressed = pressed.clone();
      move |event| match event {
        PointerEvent::Moved { x, y } => *at.lock().unwrap() = (*x, *y),
        PointerEvent::Button {
          button: PointerButton::Left,
          pressed: true,
        } => pressed.next(true),
        PointerEvent::Button {
          button: PointerButton::Left,
          pressed: false,
        } => {
          if !pressed.value() {
            return;
          }
          pressed.next(false);
          let Some(button) = this.upgrade() else { return };
          let (width, height) = button.size().value();
          let (x, y) = *at.lock().unwrap();
          if x >= 0.0 && y >= 0.0 && x < width as f32 && y < height as f32 {
            button.next(Clicked);
          }
        }
        PointerEvent::Button { .. } => {}
      }
    });

    // Enter or Space activates it while it has the keyboard.
    let this = component.downgrade();
    component.subject::<KeyEvent>().subscribe(move |event| {
      if event.pressed && (event.key == "Enter" || event.key == " ") {
        if let Some(button) = this.upgrade() {
          button.next(Clicked);
        }
      }
    });

    Self {
      component,
      label,
      text_color,
      primary,
    }
  }

  /// The clicks on this button as a stream, to derive state from without a handler:
  /// `button.subject::<Clicked>().scan(false, |on, _| !*on)` is a value that flips on every click.
  pub fn clicks(&self) -> Subject<Clicked> {
    self.component.subject::<Clicked>()
  }

  properties! {
    label: impl Into<String> => set_label, with_label;
    text_color: [u8; 4] => set_text_color, with_text_color;
  }

  /// Sets the face color for this button, whatever the theme says, and redraws. It is the button's `background`.
  pub fn set_color(&self, p_rgba: [u8; 4]) {
    self.component.set_background(Background::Color(p_rgba));
  }

  /// Sets the face color, and returns the button so calls can be chained.
  pub fn with_color(self, p_rgba: [u8; 4]) -> Self {
    self.set_color(p_rgba);
    self
  }

  /// Makes the button the accent-colored call to action (`true`) or a plain control again (`false`).
  pub fn set_primary(&self, p_primary: bool) {
    self.primary.next(p_primary);
    self.component.set_background(if p_primary { Background::Accent } else { Background::Control });
    self.component.set_border_width(if p_primary { 0 } else { 1 });
  }

  /// Makes the button primary or plain, and returns the button so calls can be chained.
  pub fn with_primary(self, p_primary: bool) -> Self {
    self.set_primary(p_primary);
    self
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  /// Gives the button the size its parent would.
  fn sized(p_button: Button, p_width: u32, p_height: u32) -> Button {
    p_button.size().next((p_width, p_height));
    p_button
  }

  fn render(p_button: &Button) -> Frame {
    let mut component = p_button.component.clone();
    component.render(Duration::ZERO)
  }

  fn pixel(p_frame: &Frame, p_x: u32, p_y: u32) -> [u8; 4] {
    let at = ((p_y * p_frame.width + p_x) * 4) as usize;
    p_frame.pixels[at..at + 4].try_into().unwrap()
  }

  fn clicks(p_button: &Button) -> Arc<Mutex<u32>> {
    let count = Arc::new(Mutex::new(0));
    p_button.subject::<Clicked>().subscribe({
      let count = Arc::clone(&count);
      move |_| *count.lock().unwrap() += 1
    });
    count
  }

  fn press(p_button: &Button, p_x: f32, p_y: f32, p_release_x: f32, p_release_y: f32) {
    let left = |pressed| PointerEvent::Button {
      button: PointerButton::Left,
      pressed,
    };
    p_button.next(PointerEvent::Moved { x: p_x, y: p_y });
    p_button.next(left(true));
    p_button.next(PointerEvent::Moved {
      x: p_release_x,
      y: p_release_y,
    });
    p_button.next(left(false));
  }

  #[test]
  fn it_draws_its_color_with_rounded_corners_and_its_label_in_the_middle() {
    let button = sized(Button::new("OK").with_color([10, 20, 30, 255]).with_text_color([250, 0, 0, 255]), 80, 30);
    let frame = render(&button);
    assert_eq!(pixel(&frame, 40, 2), [10, 20, 30, 255], "the face is the button's color");
    assert_eq!(pixel(&frame, 0, 0)[3], 0, "the corner is rounded away");
    let text_pixels = frame.pixels.chunks_exact(4).filter(|pixel| pixel[0] == 250 && pixel[1] == 0).count();
    assert!(text_pixels > 20, "the label is drawn in the text color");
    let middle_row = &frame.pixels[(15 * 80 * 4) as usize..(16 * 80 * 4) as usize];
    assert!(
      middle_row.chunks_exact(4).skip(2).take(20).all(|pixel| pixel[0] == 10),
      "the label is away from the left edge"
    );
  }

  #[test]
  fn it_is_a_plain_gray_control_with_a_border_by_default_and_the_accent_when_primary() {
    let button = sized(Button::new("OK"), 80, 30);
    let theme = Theme::light();
    let frame = render(&button);
    assert_eq!(pixel(&frame, 40, 5), theme.control, "the theme's control face");
    assert_eq!(pixel(&frame, 40, 0), theme.border, "with a thin border");
    assert_eq!(pixel(&frame, 0, 0)[3], 0, "and rounded corners");
    let dark_text = frame.pixels.chunks_exact(4).filter(|pixel| pixel[..3] == theme.control_text[..3]).count();
    assert!(dark_text > 20, "its label is in the theme's control text color");

    button.set_primary(true);
    let frame = render(&button);
    assert_eq!(pixel(&frame, 40, 5), theme.accent, "primary is the accent");
    assert_eq!(pixel(&frame, 40, 0), theme.accent, "with no border");
    assert!(frame.pixels.chunks_exact(4).any(|pixel| pixel[..3] == theme.accent_text[..3]), "and the text made for it");
  }

  #[test]
  fn it_looks_like_the_theme_until_a_property_overrides_part_of_it() {
    let button = sized(Button::new("OK"), 80, 30);

    // A theme set on a parent reaches the button, and it redraws.
    let parent = Component::new("parent");
    parent.add(button.clone());
    render(&button);
    parent.set_theme(ThemePatch::new().control([9, 8, 7, 255]).radius(0.0));
    assert!(button.component.clone().has_changed());
    let frame = render(&button);
    assert_eq!(pixel(&frame, 40, 5), [9, 8, 7, 255]);
    assert_eq!(pixel(&frame, 0, 0), Theme::light().border, "the theme's radius of 0 leaves the corner square");

    // The button's own setting wins over the theme, for just that one thing.
    button.set_radius(12.0);
    let frame = render(&button);
    assert_eq!(pixel(&frame, 40, 5), [9, 8, 7, 255]);
    assert_eq!(pixel(&frame, 0, 0)[3], 0);
    button.set_color([1, 2, 3, 255]);
    assert_eq!(pixel(&render(&button), 40, 5), [1, 2, 3, 255]);
  }

  #[test]
  fn like_a_browser_button_it_ignores_its_parents_font_and_text_color_unless_told_to_follow() {
    let page = Component::new("page");
    page.set_font_size(40);
    page.set_theme(ThemePatch::new().text([1, 2, 3, 255]));
    let button = sized(Button::new("OK"), 80, 30);
    page.add(button.clone());

    assert_eq!(button.font().size, 16, "the host's default size, not the page's 40");
    let frame = render(&button);
    let page_colored = frame.pixels.chunks_exact(4).filter(|pixel| pixel[..3] == [1, 2, 3]).count();
    assert_eq!(page_colored, 0, "its text is the control text color, not the page's text color");

    // A setting on the button itself still works, and so does asking it to follow its parents.
    button.set_font_size(24);
    assert_eq!(button.font().size, 24);
    button.inherit_font();
    button.set_inherits_font(true);
    assert_eq!(button.font().size, 40, "like CSS `font: inherit`");
  }

  #[test]
  fn changing_a_property_redraws_it() {
    let button = sized(Button::new("A"), 40, 20);
    render(&button);
    assert!(!button.component.clone().has_changed(), "drawn and unchanged");
    button.set_color([1, 2, 3, 255]);
    assert!(button.component.clone().has_changed());
    assert_eq!(pixel(&render(&button), 20, 1), [1, 2, 3, 255]);
    button.set_label("Longer");
    assert!(button.component.clone().has_changed());
  }

  #[test]
  fn pressing_and_letting_go_over_it_is_one_click_and_it_looks_pressed_meanwhile() {
    let button = sized(Button::new("Go").with_color([200, 200, 200, 255]), 60, 20);
    let count = clicks(&button);

    button.next(PointerEvent::Moved { x: 30.0, y: 10.0 });
    button.next(PointerEvent::Button {
      button: PointerButton::Left,
      pressed: true,
    });
    assert!(pixel(&render(&button), 30, 1)[0] < 200, "darker while pressed");
    assert_eq!(*count.lock().unwrap(), 0, "not clicked until it is let go");
    button.next(PointerEvent::Button {
      button: PointerButton::Left,
      pressed: false,
    });
    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(pixel(&render(&button), 30, 1)[0], 200, "normal again");
  }

  #[test]
  fn letting_go_outside_it_or_with_another_button_is_not_a_click() {
    let button = sized(Button::new("Go"), 60, 20);
    let count = clicks(&button);

    press(&button, 30.0, 10.0, 90.0, 10.0); // dragged off to the right and let go
    assert_eq!(*count.lock().unwrap(), 0);

    button.next(PointerEvent::Button {
      button: PointerButton::Right,
      pressed: true,
    });
    button.next(PointerEvent::Button {
      button: PointerButton::Right,
      pressed: false,
    });
    assert_eq!(*count.lock().unwrap(), 0);

    press(&button, 5.0, 5.0, 6.0, 6.0);
    assert_eq!(*count.lock().unwrap(), 1);
  }

  #[test]
  fn enter_and_space_click_it_and_other_keys_do_not() {
    let button = sized(Button::new("Go"), 60, 20);
    let count = clicks(&button);
    let key = |name: &str, pressed| KeyEvent {
      key: name.to_string(),
      pressed,
    };
    button.next(key("Enter", true));
    button.next(key("Enter", false));
    button.next(key(" ", true));
    button.next(key("a", true));
    assert_eq!(*count.lock().unwrap(), 2);
  }

  #[test]
  fn two_buttons_keep_their_own_state() {
    let (first, second) = (sized(Button::new("One"), 10, 10), sized(Button::new("Two"), 10, 10));
    let first_count = clicks(&first);
    let second_count = clicks(&second);
    press(&first, 5.0, 5.0, 5.0, 5.0);
    assert_eq!((*first_count.lock().unwrap(), *second_count.lock().unwrap()), (1, 0));
  }

  #[test]
  fn a_button_can_be_added_to_a_parent_and_dropping_it_frees_it() {
    let parent = Component::new("parent").with_display(Display::Flex).with_size(100, 20);
    let button = Button::new("Go");
    parent.add(button.clone().width(40)).add(Component::new("rest"));
    parent.clone().render(Duration::ZERO);
    assert_eq!(button.size().value(), (40, 20), "it is placed like any component");

    let weak = button.downgrade();
    drop(button);
    drop(parent);
    assert!(weak.upgrade().is_none(), "its own listeners do not keep it alive");
  }
}
