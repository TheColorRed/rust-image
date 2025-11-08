//! Anchor points and positioning logic for layers and canvases.

/// Anchor points for positioning elements within a parent container.
///
/// Anchor points define where an element should be positioned relative to its parent.
/// For example, `Center` positions the element in the center, while `TopLeft` positions
/// it at the top-left corner.
#[derive(Clone, Copy)]
pub enum Anchor {
  /// Anchors the element to the top left corner of the parent.
  TopLeft,
  /// Anchors the element to the top center of the parent.
  TopCenter,
  /// Anchors the element to the top right corner of the parent.
  TopRight,
  /// Anchors the element to the center left of the parent.
  CenterLeft,
  /// Anchors the element to the center of the parent.
  Center,
  /// Anchors the element to the center right of the parent.
  CenterRight,
  /// Anchors the element to the bottom left corner of the parent.
  BottomLeft,
  /// Anchors the element to the bottom center of the parent.
  BottomCenter,
  /// Anchors the element to the bottom right corner of the parent.
  BottomRight,
}

impl Anchor {
  /// Calculates the position (x, y) for this anchor point given parent and child dimensions.
  ///
  /// This method determines where a child element should be positioned within a parent
  /// based on the anchor point and the dimensions of both the parent and child.
  ///
  /// # Arguments
  /// * `parent_width` - The width of the parent container
  /// * `parent_height` - The height of the parent container
  /// * `child_width` - The width of the child element
  /// * `child_height` - The height of the child element
  ///
  /// # Returns
  /// A tuple (x, y) representing the position of the child within the parent
  ///
  /// # Example
  /// ```ignore
  /// let (x, y) = Anchor::Center.calculate_position(1000, 800, 100, 50);
  /// // x = 450, y = 375 (centers the 100x50 element in a 1000x800 parent)
  /// ```
  pub(crate) fn calculate_position(self, parent_width: i32, parent_height: i32, child_width: i32, child_height: i32) -> (i32, i32) {
    let x = match self {
      Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => 0,
      Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => (parent_width - child_width) / 2,
      Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => parent_width - child_width,
    };
    let y = match self {
      Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => 0,
      Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => (parent_height - child_height) / 2,
      Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => parent_height - child_height,
    };
    (x, y)
  }
}
