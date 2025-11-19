//! Origin points for layers that define the anchor point within the layer's image.

/// Origin points that define where within a layer's image the anchor point should be.
///
/// The origin determines which point of the layer will be positioned according to the Anchor.
/// For example, `Center` means the layer's center point will be positioned at the canvas anchor,
/// while `TopLeft` means the layer's top-left corner will be positioned at the canvas anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Origin {
  /// The anchor point is at the top left of the layer.
  TopLeft,
  /// The anchor point is at the top center of the layer.
  TopCenter,
  /// The anchor point is at the top right of the layer.
  TopRight,
  /// The anchor point is at the center left of the layer.
  CenterLeft,
  /// The anchor point is at the center of the layer (default).
  Center,
  /// The anchor point is at the center right of the layer.
  CenterRight,
  /// The anchor point is at the bottom left of the layer.
  BottomLeft,
  /// The anchor point is at the bottom center of the layer.
  BottomCenter,
  /// The anchor point is at the bottom right of the layer.
  BottomRight,
  /// The anchor point is at a custom percentage position (x, y) where 0.0 = left/top and 1.0 = right/bottom.
  /// For example, Position(0.5, 0.5) is the same as Center.
  Custom(f32, f32),
}

impl Origin {
  /// Calculates the offset from the layer's top-left to the origin point.
  ///
  /// Returns the (x, y) offset where the anchor point is located within the layer.
  ///
  /// # Arguments
  /// * `width` - The width of the layer
  /// * `height` - The height of the layer
  ///
  /// # Returns
  /// A tuple (x, y) representing the offset from top-left to the origin point
  #[allow(dead_code)]
  pub(crate) fn calculate_offset(self, width: i32, height: i32) -> (i32, i32) {
    let x = match self {
      Origin::TopLeft | Origin::CenterLeft | Origin::BottomLeft => 0,
      Origin::TopCenter | Origin::Center | Origin::BottomCenter => width / 2,
      Origin::TopRight | Origin::CenterRight | Origin::BottomRight => width,
      Origin::Custom(px, _) => ((width as f32) * px) as i32,
    };
    let y = match self {
      Origin::TopLeft | Origin::TopCenter | Origin::TopRight => 0,
      Origin::CenterLeft | Origin::Center | Origin::CenterRight => height / 2,
      Origin::BottomLeft | Origin::BottomCenter | Origin::BottomRight => height,
      Origin::Custom(_, py) => ((height as f32) * py) as i32,
    };
    (x, y)
  }
}

impl Default for Origin {
  fn default() -> Self {
    Origin::Center
  }
}
