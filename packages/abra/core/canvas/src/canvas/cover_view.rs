/// How a picture is zoomed, turned and moved inside a frame it has to cover, as when a photo sits in a cell of a collage.
///
/// A zoom of 1 is the smallest the picture can be and still cover the frame. The picture is never allowed smaller than what still covers
/// the frame at the turn it has, and never moved further than it still covers, so there are no empty corners however it is set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoverView {
  /// How far the picture is zoomed in from the smallest size that covers the frame. 1 is that size.
  pub zoom: f32,
  /// How far the picture is turned about its middle, in degrees. A positive turn is clockwise.
  pub turn: f32,
  /// How far the picture is moved from the middle of the frame, in pixels of the frame.
  pub pan: (f32, f32),
}

impl Default for CoverView {
  fn default() -> Self {
    CoverView { zoom: 1.0, turn: 0.0, pan: (0.0, 0.0) }
  }
}

/// Where a picture sits in its frame for a view: the factor it is scaled by about its middle, its turn and the position of its top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CoverPlacement {
  pub scale: f32,
  pub rotation: Option<f32>,
  pub position: (i32, i32),
}

impl CoverView {
  /// Creates a view that shows the picture as it just covers the frame.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets how far the picture is zoomed in from the smallest size that covers the frame.
  pub fn with_zoom(mut self, p_zoom: f32) -> Self {
    self.zoom = p_zoom;
    self
  }

  /// Sets how far the picture is turned about its middle, in degrees.
  pub fn with_turn(mut self, p_degrees: f32) -> Self {
    self.turn = p_degrees;
    self
  }

  /// Sets how far the picture is moved from the middle of the frame, in pixels of the frame.
  pub fn with_pan(mut self, p_x: f32, p_y: f32) -> Self {
    self.pan = (p_x, p_y);
    self
  }

  /// Where a picture of `p_picture` pixels sits in a frame of `p_frame` pixels for this view. The picture keeps its own pixels and is
  /// placed by its scale, turn and position, so setting a view again never loses quality.
  pub(crate) fn placement(&self, p_picture: (f32, f32), p_frame: (f32, f32)) -> CoverPlacement {
    let ((picture_width, picture_height), (frame_width, frame_height)) = (p_picture, p_frame);
    let rotation = (self.turn != 0.0).then_some(self.turn);
    if picture_width <= 0.0 || picture_height <= 0.0 {
      return CoverPlacement { scale: self.zoom.max(0.0), rotation, position: (0, 0) };
    }
    let (sin, cos) = self.turn.to_radians().sin_cos();
    // How much of the picture's own width and height the frame needs at this turn.
    let reach = (frame_width * cos.abs() + frame_height * sin.abs(), frame_width * sin.abs() + frame_height * cos.abs());
    let covering = (frame_width / picture_width).max(frame_height / picture_height);
    let scale = (covering * self.zoom).max(reach.0 / picture_width).max(reach.1 / picture_height);
    // The picture may be moved only as far as it still covers the frame: that far along its own axes, which are turned with it.
    let local = (cos * self.pan.0 + sin * self.pan.1, -sin * self.pan.0 + cos * self.pan.1);
    let slack = (((scale * picture_width - reach.0) / 2.0).max(0.0), ((scale * picture_height - reach.1) / 2.0).max(0.0));
    let kept = (local.0.clamp(-slack.0, slack.0), local.1.clamp(-slack.1, slack.1));
    let middle = (frame_width / 2.0 + cos * kept.0 - sin * kept.1, frame_height / 2.0 + sin * kept.0 + cos * kept.1);
    CoverPlacement {
      scale,
      rotation,
      position: ((middle.0 - picture_width / 2.0).round() as i32, (middle.1 - picture_height / 2.0).round() as i32),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zoom_one_just_covers_the_frame_and_sits_in_the_middle() {
    let placed = CoverView::new().placement((200.0, 100.0), (100.0, 100.0));
    assert_eq!(placed.scale, 1.0);
    assert_eq!(placed.rotation, None);
    assert_eq!(placed.position, (-50, 0));
  }

  #[test]
  fn a_turn_makes_the_picture_big_enough_to_leave_no_empty_corners() {
    let placed = CoverView::new().with_turn(45.0).placement((100.0, 100.0), (100.0, 100.0));
    assert!((placed.scale - 2.0_f32.sqrt()).abs() < 1e-4);
    assert_eq!(placed.rotation, Some(45.0));
  }

  #[test]
  fn the_picture_cannot_be_moved_past_what_still_covers_the_frame() {
    // Zoomed to twice the size it has 50 pixels of slack each way.
    let far = CoverView::new().with_zoom(2.0).with_pan(500.0, -500.0).placement((100.0, 100.0), (100.0, 100.0));
    assert_eq!(far.position, (50, -50));
    let near = CoverView::new().with_zoom(2.0).with_pan(20.0, 0.0).placement((100.0, 100.0), (100.0, 100.0));
    assert_eq!(near.position, (20, 0));
  }

  #[test]
  fn a_picture_that_just_covers_cannot_be_moved() {
    let placed = CoverView::new().with_pan(30.0, 30.0).placement((100.0, 100.0), (100.0, 100.0));
    assert_eq!(placed.position, (0, 0));
  }

  #[test]
  fn a_move_follows_the_turn_of_the_picture() {
    // Turned a quarter, the picture's own x axis points down the frame, so a move down is a move along it.
    let placed = CoverView::new().with_zoom(2.0).with_turn(90.0).with_pan(0.0, 30.0).placement((100.0, 100.0), (100.0, 100.0));
    assert_eq!(placed.position, (0, 30));
  }
}
