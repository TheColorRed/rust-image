use std::time::Duration;

use vessel_api::prelude::*;

use crate::image::Viewport;

/// A viewport that glides to wherever it is told to go, like a game camera. Whoever drives it sets the target (from fingers,
/// a mouse or a script) and calls [`step`](Self::step) once per frame with the time since the last one. The speeds live here
/// so every view moves the same way: size changes ease in log space, so a zoom feels even, and the center eases in a line
/// (see the [`Animatable`] impl of [`Viewport`]).
///
/// A move that should take a fixed time is a tween instead: `component.animate(&viewport, target, options)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
  current: Viewport,
  target: Viewport,
  /// How fast it closes in on the target. Each second it covers `1 - e^-rate` of what is left, so higher is snappier; the
  /// default 18 gets most of the way in about a fifth of a second.
  pub rate: f32,
}

impl Camera {
  /// A camera sitting on `p_view`, with the default speed.
  pub fn new(p_view: Viewport) -> Self {
    Self { current: p_view, target: p_view, rate: 18.0 }
  }

  /// Where the camera is now.
  pub fn viewport(&self) -> Viewport {
    self.current
  }

  /// Where the camera is heading.
  pub fn target(&self) -> Viewport {
    self.target
  }

  /// Sends the camera toward `p_view`.
  pub fn set_target(&mut self, p_view: Viewport) {
    self.target = p_view;
  }

  /// Puts the camera on `p_view` at once, with no gliding.
  pub fn jump_to(&mut self, p_view: Viewport) {
    self.current = p_view;
    self.target = p_view;
  }

  /// Whether the camera has not reached its target, so another frame is needed.
  pub fn is_moving(&self) -> bool {
    self.current != self.target
  }

  /// Moves the camera toward its target for `p_delta` of time and returns where it is now. It lands exactly on the target
  /// once it is very close (a ten-thousandth), so it always stops.
  pub fn step(&mut self, p_delta: Duration) -> Viewport {
    let share = 1.0 - (-self.rate * p_delta.as_secs_f32()).exp();
    let (from, to) = (self.current, self.target);
    let next = Viewport::lerp(&from, &to, share);
    let near = |a: f32, b: f32| (a - b).abs() <= 1e-6 + b.abs() * 1e-4;
    self.current = if near(next.x, to.x) && near(next.y, to.y) && near(next.width, to.width) && near(next.height, to.height) {
      to
    } else {
      next
    };
    self.current
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn view(x: f32, y: f32, size: f32) -> Viewport {
    Viewport { x, y, width: size, height: size }
  }

  #[test]
  fn it_glides_toward_the_target_and_lands_on_it() {
    let mut camera = Camera::new(Viewport::default());
    camera.set_target(view(0.4, 0.4, 0.2));
    assert!(camera.is_moving());
    let first = camera.step(Duration::from_millis(16));
    assert!(first.width < 1.0 && first.width > 0.2, "one frame moves part of the way: {first:?}");
    for _ in 0..200 {
      camera.step(Duration::from_millis(16));
    }
    assert_eq!(camera.viewport(), view(0.4, 0.4, 0.2));
    assert!(!camera.is_moving());
  }

  #[test]
  fn zoom_eases_evenly_and_the_center_stays_between_start_and_target() {
    let mut camera = Camera::new(Viewport::default());
    camera.set_target(view(0.45, 0.45, 0.1));
    let a = camera.step(Duration::from_millis(16));
    let b = camera.step(Duration::from_millis(16));
    let left = |width: f32| (width / 0.1).ln();
    let (first, second) = (left(a.width) / left(1.0), left(b.width) / left(a.width));
    assert!((first - second).abs() < 1e-4, "each frame closes the same share of the zoom: {first} vs {second}");
    assert!((a.x + a.width / 2.0 - 0.5).abs() < 1e-5, "zooming on the middle keeps the middle");
  }

  #[test]
  fn a_long_frame_does_not_overshoot_and_jump_to_is_immediate() {
    let mut camera = Camera::new(Viewport::default());
    camera.set_target(view(0.2, 0.2, 0.5));
    let landed = camera.step(Duration::from_secs(10));
    assert_eq!(landed, view(0.2, 0.2, 0.5));
    camera.jump_to(Viewport::default());
    assert_eq!(camera.viewport(), Viewport::default());
    assert!(!camera.is_moving());
  }

  #[test]
  fn a_viewport_tween_zooms_evenly_along_its_whole_way() {
    let mut tween = Tween::new(Viewport::default(), view(0.45, 0.45, 0.1), TweenOptions::default().with_duration(Duration::from_millis(100)).with_easing(Easing::Linear));
    tween.step(Duration::ZERO);
    let half = tween.step(Duration::from_millis(50));
    assert!((half.width - 0.1f32.sqrt()).abs() < 1e-4, "half the time is half the zoom in log space: {half:?}");
    assert!((half.x + half.width / 2.0 - 0.5).abs() < 1e-5, "zooming on the middle keeps the middle");
    assert_eq!(tween.step(Duration::from_millis(60)), view(0.45, 0.45, 0.1));
  }
}
