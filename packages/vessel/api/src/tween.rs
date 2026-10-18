//! Tweens: moving a value to a new one over a fixed time.
//!
//! [`Tween`] is the plain maths: give it a start, a target and [`TweenOptions`], feed it the time of each frame, and it says
//! where the value is. [`Component::animate`](crate::Component::animate) is how a component uses one: it takes any
//! `BehaviorSubject` it owns (a color, an opacity, a camera, a position) and moves it to a target over the next frames, so
//! nothing that animates has to count time itself.

use std::time::Duration;

/// A value a tween can move: it can be blended part of the way from one value to another.
pub trait Animatable: Clone + Send + Sync + 'static {
  /// The value `p_share` of the way from `p_from` to `p_to`: `0.0` is `p_from`, `1.0` is `p_to`.
  fn lerp(p_from: &Self, p_to: &Self, p_share: f32) -> Self;
}

impl Animatable for f32 {
  fn lerp(p_from: &Self, p_to: &Self, p_share: f32) -> Self {
    p_from + (p_to - p_from) * p_share
  }
}

impl Animatable for f64 {
  fn lerp(p_from: &Self, p_to: &Self, p_share: f32) -> Self {
    p_from + (p_to - p_from) * p_share as f64
  }
}

impl Animatable for (f32, f32) {
  fn lerp(p_from: &Self, p_to: &Self, p_share: f32) -> Self {
    (f32::lerp(&p_from.0, &p_to.0, p_share), f32::lerp(&p_from.1, &p_to.1, p_share))
  }
}

/// A color, each channel blended on its own.
impl Animatable for [u8; 4] {
  fn lerp(p_from: &Self, p_to: &Self, p_share: f32) -> Self {
    std::array::from_fn(|i| f32::lerp(&(p_from[i] as f32), &(p_to[i] as f32), p_share).round().clamp(0.0, 255.0) as u8)
  }
}

/// How a timed move speeds up and slows down. `apply` turns the share of the time that has passed (0 to 1) into the share of
/// the distance that has been covered (0 to 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
  /// The same speed all the way.
  Linear,
  /// Starts fast and slows down to the end.
  EaseOut,
  /// Starts slowly, is fastest in the middle and slows down to the end.
  #[default]
  EaseInOut,
}

impl Easing {
  /// The share of the distance covered after the share `p_time` of the time.
  pub fn apply(self, p_time: f32) -> f32 {
    let t = p_time.clamp(0.0, 1.0);
    match self {
      Easing::Linear => t,
      Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
      Easing::EaseInOut if t < 0.5 => 4.0 * t * t * t,
      Easing::EaseInOut => 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0,
    }
  }
}

/// How long a tween takes and how it eases. The default is a third of a second, easing in and out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TweenOptions {
  /// How long the move takes.
  pub duration: Duration,
  /// How it speeds up and slows down.
  pub easing: Easing,
}

impl Default for TweenOptions {
  fn default() -> Self {
    Self { duration: Duration::from_millis(300), easing: Easing::default() }
  }
}

impl TweenOptions {
  /// These options with a different duration.
  pub fn with_duration(self, p_duration: Duration) -> Self {
    Self { duration: p_duration, ..self }
  }

  /// These options with a different easing.
  pub fn with_easing(self, p_easing: Easing) -> Self {
    Self { easing: p_easing, ..self }
  }
}

/// A value on its way from a start to a target. Feed it the time of each frame with [`step`](Self::step).
#[derive(Clone, Debug, PartialEq)]
pub struct Tween<T: Animatable> {
  from: T,
  to: T,
  options: TweenOptions,
  passed: Duration,
  /// True until the first step. The time of the first frame covers everything since the engine last woke (it sleeps while
  /// nothing moves, so a view can have been idle for seconds), which is not time the move has had, so it does not count.
  fresh: bool,
}

impl<T: Animatable> Tween<T> {
  /// A tween from `p_from` to `p_to`.
  pub fn new(p_from: T, p_to: T, p_options: TweenOptions) -> Self {
    Self { from: p_from, to: p_to, options: p_options, passed: Duration::ZERO, fresh: true }
  }

  /// Moves on by `p_delta` (the time since the last frame) and returns where the value is now. It is exactly the target once
  /// the duration has passed.
  pub fn step(&mut self, p_delta: Duration) -> T {
    if !self.fresh {
      self.passed += p_delta;
    }
    self.fresh = false;
    self.value()
  }

  /// Where the value is now.
  pub fn value(&self) -> T {
    if self.is_finished() {
      return self.to.clone();
    }
    let time = if self.options.duration.is_zero() { 1.0 } else { self.passed.as_secs_f32() / self.options.duration.as_secs_f32() };
    T::lerp(&self.from, &self.to, self.options.easing.apply(time))
  }

  /// Where the value is going.
  pub fn target(&self) -> &T {
    &self.to
  }

  /// Whether the value has reached the target. A tween that has not been stepped yet is not finished, whatever its duration,
  /// so the target is always seen on a frame of its own.
  pub fn is_finished(&self) -> bool {
    !self.fresh && self.passed >= self.options.duration
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_tween_takes_exactly_its_time_and_lands_on_the_target() {
    let mut tween = Tween::new(0.0f32, 10.0, TweenOptions::default().with_duration(Duration::from_millis(100)).with_easing(Easing::Linear));
    assert!(!tween.is_finished());
    // The first tick covers the time the engine slept, which must not eat into the move.
    assert_eq!(tween.step(Duration::from_secs(5)), 0.0);
    assert_eq!(tween.step(Duration::from_millis(50)), 5.0);
    assert!(!tween.is_finished());
    assert_eq!(tween.step(Duration::from_millis(60)), 10.0);
    assert!(tween.is_finished());
  }

  #[test]
  fn easing_starts_at_zero_and_ends_at_one() {
    for easing in [Easing::Linear, Easing::EaseOut, Easing::EaseInOut] {
      assert_eq!((easing.apply(0.0), easing.apply(1.0)), (0.0, 1.0), "{easing:?}");
    }
    assert!(Easing::EaseInOut.apply(0.25) < 0.25 && Easing::EaseOut.apply(0.25) > 0.25);
  }

  #[test]
  fn a_zero_length_tween_lands_on_its_first_frame() {
    let mut tween = Tween::new(1.0f32, 2.0, TweenOptions::default().with_duration(Duration::ZERO));
    assert!(!tween.is_finished());
    assert_eq!(tween.step(Duration::from_millis(16)), 2.0);
    assert!(tween.is_finished());
  }

  #[test]
  fn colors_and_points_blend_each_part() {
    assert_eq!(<[u8; 4]>::lerp(&[0, 100, 200, 255], &[100, 200, 0, 255], 0.5), [50, 150, 100, 255]);
    assert_eq!(<(f32, f32)>::lerp(&(0.0, 10.0), &(10.0, 0.0), 0.25), (2.5, 7.5));
  }
}
