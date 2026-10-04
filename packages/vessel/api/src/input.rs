//! Input and requests in terms every platform can fill in or understand.
//!
//! A host (a desktop window, a native Android activity, a React Native view) turns what its platform reports into these
//! types and sends them to the component it shows, as ordinary events: `component.next(KeyEvent { .. })`. A component
//! listens with `component.subject::<KeyEvent>()` and `component.subject::<PointerEvent>()`, or `subscribe`, and never learns which platform they
//! came from.

/// A key going down or up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
  /// The character the key types (`"a"`) or the name of a special key (`"ArrowUp"`, `"Escape"`, `"Enter"`).
  pub key: String,
  /// True when the key went down, false when it came up.
  pub pressed: bool,
}

/// A mouse button, or the touch that stands in for the primary one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
  /// The main button, or a touch.
  Left,
  /// The secondary button.
  Right,
  /// The wheel button.
  Middle,
  /// Any other button.
  Other,
}

impl PointerButton {
  /// The button a platform calls `p_name` (`"Left"`, `"Right"`, `"Middle"`), or `Other`.
  pub fn from_name(p_name: &str) -> Self {
    match p_name {
      "Left" => Self::Left,
      "Right" => Self::Right,
      "Middle" => Self::Middle,
      _ => Self::Other,
    }
  }
}

/// The pointer (a mouse, or a finger) moving or pressing. Positions are in pixels from the component's top left corner.
#[derive(Clone, Debug, PartialEq)]
pub enum PointerEvent {
  /// The pointer moved.
  Moved {
    /// Distance from the left edge.
    x: f32,
    /// Distance from the top edge.
    y: f32,
  },
  /// A button went down or up.
  Button {
    /// Which button.
    button: PointerButton,
    /// True when it went down.
    pressed: bool,
  },
}

/// A request from a component to end the app. The host listens for it on the component it shows, closes its window or
/// activity, and returns. Send it with `component.next(Quit)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quit;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn buttons_are_named_by_the_platform_and_unknown_ones_are_other() {
    assert_eq!(PointerButton::from_name("Left"), PointerButton::Left);
    assert_eq!(PointerButton::from_name("Right"), PointerButton::Right);
    assert_eq!(PointerButton::from_name("Middle"), PointerButton::Middle);
    assert_eq!(PointerButton::from_name("Back"), PointerButton::Other);
  }
}
