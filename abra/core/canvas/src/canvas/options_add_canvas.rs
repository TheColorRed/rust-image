use crate::Anchor;

#[derive(Clone)]
/// Additional options for adding a canvas to another canvas.
pub struct AddCanvasOptions {
  /// Anchor point for the canvas within the parent canvas.
  pub anchor: Option<Anchor>,
  /// Optional position offset (x, y) for the canvas within the parent canvas.
  pub position: Option<(i32, i32)>,
  /// Optional rotation in degrees for the canvas.
  pub rotation: Option<f32>,
}

impl Default for AddCanvasOptions {
  fn default() -> Self {
    AddCanvasOptions {
      anchor: Some(Anchor::Center),
      position: None,
      rotation: None,
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
  pub fn with_anchor(mut self, p_anchor: Anchor) -> Self {
    self.anchor = Some(p_anchor);
    self
  }

  /// Sets the position offset (x, y) for the canvas within the parent canvas.
  pub fn with_position(mut self, p_x: i32, p_y: i32) -> Self {
    self.position = Some((p_x, p_y));
    self
  }

  /// Sets the rotation in degrees for the canvas.
  pub fn with_rotation(mut self, p_degrees: f32) -> Self {
    self.rotation = Some(p_degrees);
    self
  }
}
