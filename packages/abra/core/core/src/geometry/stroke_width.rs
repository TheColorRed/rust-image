use std::fmt::{self, Display, Formatter};

use crate::IntoNumber;

use super::stroke_profile::{Stop, StrokeEasing, value_at};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// The width stops of a stroke.
pub struct WidthStop {
  /// The width of the stroke at the stop.
  pub width: f32,
  /// A value between 0 and 1 for how far along the path the stop is: `0` is the start of the path and `1` is its end.
  pub time: f32,
}

impl WidthStop {
  /// Creates a width stop with the given width and time.
  pub fn new(p_width: impl IntoNumber, p_time: impl IntoNumber) -> WidthStop {
    WidthStop {
      width: p_width.into::<f32>(),
      time: p_time.into::<f32>(),
    }
  }
}

impl Stop for WidthStop {
  fn value(&self) -> f32 {
    self.width
  }
  fn time(&self) -> f32 {
    self.time
  }
}

impl Display for WidthStop {
  /// Displays the width stop as a string.
  fn fmt(&self, p_f: &mut Formatter) -> fmt::Result {
    write!(p_f, "{} wide at {}", self.width, self.time)
  }
}

#[derive(Clone, Debug, PartialEq)]
/// How wide a stroke is along its path, set at stops the way a [`Gradient`](crate::Gradient) sets color: `0` is the start of the path and `1`
/// is its end (measured along the line, so a stop at `0.5` is halfway along its length), and the width between two stops changes from one to
/// the other. A number is a stroke of that width all the way, so what takes a `StrokeWidth` takes a number too.
///
/// ```ignore
/// let tapered = StrokeWidth::from_to(8.0, 0.0);                                     // thick at the start, a point at the end
/// let swell = StrokeWidth::new(vec![WidthStop::new(1.0, 0.0), WidthStop::new(6.0, 0.4), WidthStop::new(0.0, 1.0)]);
/// path.stroke(tapered).to_path();
/// path.stroke(4.0).to_path();                                                      // the same width the whole way
/// ```
pub struct StrokeWidth {
  /// The width stops, in order of their position along the path.
  stops: Vec<WidthStop>,
  /// How the width between two stops changes.
  easing: StrokeEasing,
}

impl StrokeWidth {
  /// Creates a stroke width with the given stops. They can be in any order; they are sorted by position, and stops at the same position keep
  /// the order they were given in, so two of them make a step. Widths below `0` count as `0`.
  pub fn new(p_stops: Vec<WidthStop>) -> StrokeWidth {
    let mut stops = p_stops;
    stops.sort_by(|a, b| a.time.total_cmp(&b.time));
    StrokeWidth {
      stops,
      easing: StrokeEasing::default(),
    }
  }

  /// A stroke that is the same width the whole way.
  pub fn constant(p_width: impl IntoNumber) -> StrokeWidth {
    StrokeWidth::new(vec![WidthStop::new(p_width.into::<f32>(), 0.0)])
  }

  /// A stroke that goes from one width at the start of the path to another at its end. A width of `0` at one end is a taper to a point.
  pub fn from_to(p_from: impl IntoNumber, p_to: impl IntoNumber) -> StrokeWidth {
    StrokeWidth::new(vec![WidthStop::new(p_from.into::<f32>(), 0.0), WidthStop::new(p_to.into::<f32>(), 1.0)])
  }

  /// Sets how the width changes between two stops. Defaults to [`StrokeEasing::Linear`].
  pub fn with_easing(mut self, p_easing: impl Into<StrokeEasing>) -> Self {
    self.easing = p_easing.into();
    self
  }

  /// How the width between two stops changes.
  pub fn easing(&self) -> StrokeEasing {
    self.easing
  }

  /// The width stops, in order of their position.
  pub fn stops(&self) -> &[WidthStop] {
    &self.stops
  }

  /// The width of the stroke `p_time` of the way along the path, from `0` at its start to `1` at its end. Before the first stop and after the
  /// last it is that stop's width, and it is never below `0`. A stroke with no stops has no width.
  pub fn width_at(&self, p_time: f32) -> f32 {
    value_at(&self.stops, self.easing, p_time).unwrap_or(0.0).max(0.0)
  }

  /// The width of the stroke when it is the same all the way along, or nothing when it changes (or has no stops).
  pub fn constant_width(&self) -> Option<f32> {
    let first = self.stops.first()?.width;
    self.stops.iter().all(|stop| stop.width == first).then_some(first.max(0.0))
  }

  /// The widest the stroke gets, at any of its stops.
  pub fn widest(&self) -> f32 {
    self.stops.iter().fold(0.0_f32, |widest, stop| widest.max(stop.width))
  }
}

impl From<f32> for StrokeWidth {
  /// A number is a stroke of that width the whole way along the path.
  fn from(p_width: f32) -> StrokeWidth {
    StrokeWidth::constant(p_width)
  }
}

// A number of any kind is a stroke of that width, so `stroke(4)` and `stroke(2.5)` both work.
crate::impl_into_number!(StrokeWidth);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_number_is_the_same_width_the_whole_way() {
    let width = StrokeWidth::from(3.0);
    assert_eq!([0.0, 0.5, 1.0].map(|time| width.width_at(time)), [3.0; 3]);
  }

  #[test]
  fn the_width_changes_from_one_stop_to_the_next() {
    let width = StrokeWidth::from_to(8.0, 0.0);
    assert_eq!(width.width_at(0.0), 8.0);
    assert_eq!(width.width_at(0.25), 6.0);
    assert_eq!(width.width_at(0.5), 4.0);
    assert_eq!(width.width_at(1.0), 0.0);
  }

  #[test]
  fn before_the_first_stop_and_after_the_last_the_width_is_held() {
    let width = StrokeWidth::new(vec![WidthStop::new(2.0, 0.25), WidthStop::new(6.0, 0.75)]);
    assert_eq!(width.width_at(0.0), 2.0);
    assert_eq!(width.width_at(1.0), 6.0);
    assert_eq!(width.width_at(0.5), 4.0);
  }

  #[test]
  fn stops_are_sorted_and_two_at_one_place_make_a_step() {
    let width = StrokeWidth::new(vec![WidthStop::new(6.0, 1.0), WidthStop::new(1.0, 0.5), WidthStop::new(5.0, 0.5), WidthStop::new(1.0, 0.0)]);
    assert_eq!(width.stops().first().map(|stop| stop.time), Some(0.0));
    // Just before the step it is on the way to the first of the two, and at the step it is the second.
    assert!((width.width_at(0.4999) - 1.0).abs() < 0.01);
    assert_eq!(width.width_at(0.5), 5.0);
  }

  #[test]
  fn smooth_easing_is_flat_at_the_stops_and_the_same_in_the_middle() {
    let linear = StrokeWidth::from_to(0.0, 10.0);
    let smooth = StrokeWidth::from_to(0.0, 10.0).with_easing(StrokeEasing::Smooth);
    assert_eq!(smooth.width_at(0.5), linear.width_at(0.5));
    assert!(smooth.width_at(0.1) < linear.width_at(0.1));
    assert!(smooth.width_at(0.9) > linear.width_at(0.9));
  }

  #[test]
  fn a_width_is_never_below_zero() {
    assert_eq!(StrokeWidth::from_to(-4.0, 2.0).width_at(0.0), 0.0);
    assert_eq!(StrokeWidth::new(Vec::new()).width_at(0.5), 0.0);
  }

  #[test]
  fn a_width_that_never_changes_is_constant() {
    assert_eq!(StrokeWidth::constant(3.0).constant_width(), Some(3.0));
    assert_eq!(StrokeWidth::new(vec![WidthStop::new(2.0, 0.0), WidthStop::new(2.0, 1.0)]).constant_width(), Some(2.0));
    assert_eq!(StrokeWidth::from_to(2.0, 0.0).constant_width(), None);
    assert_eq!(StrokeWidth::new(Vec::new()).constant_width(), None);
  }

  #[test]
  fn the_widest_stop_is_found() {
    assert_eq!(StrokeWidth::new(vec![WidthStop::new(1.0, 0.0), WidthStop::new(6.0, 0.4), WidthStop::new(0.0, 1.0)]).widest(), 6.0);
  }
}
