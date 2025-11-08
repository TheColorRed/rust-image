use crate::Anchor;

#[derive(Clone)]
/// Additional options for adding a canvas to another canvas.
pub struct AddCanvasOptions {
  /// Anchor point for the canvas within the parent canvas.
  pub anchor: Option<Anchor>,
}

impl Default for AddCanvasOptions {
  fn default() -> Self {
    AddCanvasOptions {
      anchor: Some(Anchor::Center),
    }
  }
}

impl AddCanvasOptions {
  /// Creates a new `AddCanvasOptions` with default values.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the anchor point for the canvas.
  /// The anchor point determines how the canvas is positioned within the parent canvas when drawn.
  pub fn with_anchor(mut self, anchor: Anchor) -> Self {
    self.anchor = Some(anchor);
    self
  }
}
