use std::fmt::{self, Display, Formatter};

use crate::IntoNumber;

use super::stroke_profile::{Stop, StrokeEasing, value_at};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// The opacity stops of a stroke.
pub struct OpacityStop {
  /// The opacity of the stroke at the stop, from `0` (clear) to `1` (solid).
  pub opacity: f32,
  /// A value between 0 and 1 for how far along the path the stop is: `0` is the start of the path and `1` is its end.
  pub time: f32,
}

impl OpacityStop {
  /// Creates an opacity stop with the given opacity and time.
  pub fn new(p_opacity: impl IntoNumber, p_time: impl IntoNumber) -> OpacityStop {
    OpacityStop {
      opacity: p_opacity.into::<f32>(),
      time: p_time.into::<f32>(),
    }
  }
}

impl Stop for OpacityStop {
  fn value(&self) -> f32 {
    self.opacity
  }
  fn time(&self) -> f32 {
    self.time
  }
}

impl Display for OpacityStop {
  /// Displays the opacity stop as a string.
  fn fmt(&self, p_f: &mut Formatter) -> fmt::Result {
    write!(p_f, "{} opaque at {}", self.opacity, self.time)
  }
}

#[derive(Clone, Debug, PartialEq)]
/// How opaque a stroke is along its path, set at stops the same way as [`StrokeWidth`](crate::StrokeWidth): `0` is the start of the path and
/// `1` is its end (measured along the line, so a stop at `0.5` is halfway along its length), and the opacity between two stops changes from one
/// to the other. A number is a stroke of that opacity all the way, so what takes a `StrokeOpacity` takes a number too. Without any stops a
/// stroke is solid, which is also what a brush has until it is given an opacity.
///
/// ```ignore
/// let fade_out = StrokeOpacity::from_to(1.0, 0.0);                                   // solid at the start, clear at the end
/// let fade_in_and_out = StrokeOpacity::new(vec![OpacityStop::new(0.0, 0.0), OpacityStop::new(1.0, 0.3), OpacityStop::new(0.0, 1.0)]);
/// let brush = Brush::new().with_opacity(fade_out);
/// let half = Brush::new().with_opacity(0.5);                                         // the same opacity the whole way
/// ```
pub struct StrokeOpacity {
  /// The opacity stops, in order of their position along the path.
  stops: Vec<OpacityStop>,
  /// How the opacity between two stops changes.
  easing: StrokeEasing,
}

impl Default for StrokeOpacity {
  /// Solid all the way.
  fn default() -> StrokeOpacity {
    StrokeOpacity::constant(1.0)
  }
}

impl StrokeOpacity {
  /// Creates a stroke opacity with the given stops. They can be in any order; they are sorted by position, and stops at the same position keep
  /// the order they were given in, so two of them make a step. An opacity outside `0` to `1` is held to it where it is read.
  pub fn new(p_stops: Vec<OpacityStop>) -> StrokeOpacity {
    let mut stops = p_stops;
    stops.sort_by(|a, b| a.time.total_cmp(&b.time));
    StrokeOpacity {
      stops,
      easing: StrokeEasing::default(),
    }
  }

  /// A stroke that is the same opacity the whole way.
  pub fn constant(p_opacity: impl IntoNumber) -> StrokeOpacity {
    StrokeOpacity::new(vec![OpacityStop::new(p_opacity.into::<f32>(), 0.0)])
  }

  /// A stroke that goes from one opacity at the start of the path to another at its end. `from_to(1, 0)` fades out.
  pub fn from_to(p_from: impl IntoNumber, p_to: impl IntoNumber) -> StrokeOpacity {
    StrokeOpacity::new(vec![OpacityStop::new(p_from.into::<f32>(), 0.0), OpacityStop::new(p_to.into::<f32>(), 1.0)])
  }

  /// Sets how the opacity between two stops changes. Defaults to [`StrokeEasing::Linear`].
  pub fn with_easing(mut self, p_easing: impl Into<StrokeEasing>) -> Self {
    self.easing = p_easing.into();
    self
  }

  /// How the opacity between two stops changes.
  pub fn easing(&self) -> StrokeEasing {
    self.easing
  }

  /// The opacity stops, in order of their position.
  pub fn stops(&self) -> &[OpacityStop] {
    &self.stops
  }

  /// The opacity of the stroke `p_time` of the way along the path, from `0` at its start to `1` at its end, held to `0..=1`. Before the first
  /// stop and after the last it is that stop's opacity, and a stroke with no stops is solid (`1`).
  pub fn opacity_at(&self, p_time: f32) -> f32 {
    value_at(&self.stops, self.easing, p_time).unwrap_or(1.0).clamp(0.0, 1.0)
  }

  /// The opacity of the stroke when it is the same all the way along, or nothing when it changes. A stroke with no stops is solid.
  pub fn constant_opacity(&self) -> Option<f32> {
    let Some(first) = self.stops.first() else {
      return Some(1.0);
    };
    self.stops.iter().all(|stop| stop.opacity == first.opacity).then_some(first.opacity.clamp(0.0, 1.0))
  }
}

impl From<f32> for StrokeOpacity {
  /// A number is a stroke of that opacity the whole way along the path.
  fn from(p_opacity: f32) -> StrokeOpacity {
    StrokeOpacity::constant(p_opacity)
  }
}

// A number of any kind is a stroke of that opacity, so `with_opacity(1)` and `with_opacity(0.5)` both work.
crate::impl_into_number!(StrokeOpacity);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_number_is_the_same_opacity_the_whole_way() {
    let opacity = StrokeOpacity::from(0.4);
    assert_eq!([0.0, 0.5, 1.0].map(|time| opacity.opacity_at(time)), [0.4; 3]);
    assert_eq!(opacity.constant_opacity(), Some(0.4));
  }

  #[test]
  fn a_stroke_is_solid_unless_it_is_given_an_opacity() {
    assert_eq!(StrokeOpacity::default().opacity_at(0.5), 1.0);
    assert_eq!(StrokeOpacity::new(Vec::new()).opacity_at(0.5), 1.0);
    assert_eq!(StrokeOpacity::new(Vec::new()).constant_opacity(), Some(1.0));
  }

  #[test]
  fn the_opacity_changes_from_one_stop_to_the_next() {
    let fade = StrokeOpacity::from_to(1.0, 0.0);
    assert_eq!([0.0, 0.25, 0.5, 1.0].map(|time| fade.opacity_at(time)), [1.0, 0.75, 0.5, 0.0]);
    assert_eq!(fade.constant_opacity(), None);
  }

  #[test]
  fn before_the_first_stop_and_after_the_last_the_opacity_is_held() {
    let opacity = StrokeOpacity::new(vec![OpacityStop::new(0.2, 0.25), OpacityStop::new(0.8, 0.75)]);
    assert_eq!(opacity.opacity_at(0.0), 0.2);
    assert_eq!(opacity.opacity_at(1.0), 0.8);
    assert_eq!(opacity.opacity_at(0.5), 0.5);
  }

  #[test]
  fn stops_are_sorted_and_two_at_one_place_make_a_step() {
    let opacity = StrokeOpacity::new(vec![OpacityStop::new(1.0, 1.0), OpacityStop::new(0.0, 0.5), OpacityStop::new(0.6, 0.5), OpacityStop::new(0.0, 0.0)]);
    assert_eq!(opacity.stops().first().map(|stop| stop.time), Some(0.0));
    assert!(opacity.opacity_at(0.4999).abs() < 0.01);
    assert_eq!(opacity.opacity_at(0.5), 0.6);
  }

  #[test]
  fn an_opacity_is_held_to_zero_and_one() {
    assert_eq!(StrokeOpacity::from_to(-1.0, 2.0).opacity_at(0.0), 0.0);
    assert_eq!(StrokeOpacity::from_to(-1.0, 2.0).opacity_at(1.0), 1.0);
    assert_eq!(StrokeOpacity::constant(1).opacity_at(0.3), 1.0);
  }

  #[test]
  fn smooth_easing_is_flat_at_the_stops_and_the_same_in_the_middle() {
    let linear = StrokeOpacity::from_to(0.0, 1.0);
    let smooth = StrokeOpacity::from_to(0.0, 1.0).with_easing(StrokeEasing::Smooth);
    assert_eq!(smooth.opacity_at(0.5), linear.opacity_at(0.5));
    assert!(smooth.opacity_at(0.1) < linear.opacity_at(0.1));
  }
}
