use crate::Curve;

/// How many pieces a life is cut into when a range's area is worked out.
const AREA_STEPS: usize = 256;

/// A value that follows a curve over a life, like Unity's "random between two curves". Each thing that uses it picks one place between
/// the `min` and `max` curves and keeps it, so no two are alike but each one still changes over its life. A curve's `x` is the age from
/// `0` (birth) to `255` (end of life) and its `y` is the value. A number is a flat curve, `(0.0, 1.0)` is a flat range, a [`Curve`] is
/// a range with no spread, and `(Curve, Curve)` is the full thing, so any of them work wherever a range is asked for.
#[derive(Clone, Debug)]
pub struct CurveRange {
  min: Curve,
  max: Curve,
  /// The area under each curve from birth up to each of `AREA_STEPS + 1` evenly spaced ages, worked out once here so that a velocity
  /// can be added up over a life by looking it up instead of walking the curve.
  min_area: Vec<f32>,
  max_area: Vec<f32>,
  /// Both curves are zero all the way along.
  zero: bool,
}

impl CurveRange {
  pub fn new(p_min: Curve, p_max: Curve) -> Self {
    let area = |curve: &Curve| {
      let mut total = 0.0;
      let mut previous = curve.value(0.0);
      let mut table = vec![0.0];
      for step in 1..=AREA_STEPS {
        let value = curve.value(step as f32 / AREA_STEPS as f32 * 255.0);
        total += (previous + value) * 0.5 / AREA_STEPS as f32;
        previous = value;
        table.push(total);
      }
      table
    };
    let zero = |curve: &Curve| (0..=255).all(|x| curve.value(x as f32) == 0.0);
    CurveRange { min_area: area(&p_min), max_area: area(&p_max), zero: zero(&p_min) && zero(&p_max), min: p_min, max: p_max }
  }

  /// Whether the value is zero at every age, for every pick.
  pub fn is_zero(&self) -> bool {
    self.zero
  }

  /// The value at `p_age` (0.0 to 1.0) for the pick `p_pick` (0.0 for `min`, 1.0 for `max`).
  pub fn at(&self, p_age: f32, p_pick: f32) -> f32 {
    let (low, high) = (self.min.value(p_age * 255.0), self.max.value(p_age * 255.0));
    low + (high - low) * p_pick
  }

  /// The value added up from birth to `p_age`, for a range that is a velocity. The area under a curve between two others is the same mix
  /// of their areas, so the picked area is read from the two tables.
  pub fn area(&self, p_age: f32, p_pick: f32) -> f32 {
    let position = p_age.clamp(0.0, 1.0) * AREA_STEPS as f32;
    let (step, fraction) = (position as usize, position.fract());
    let next = (step + 1).min(AREA_STEPS);
    let read = |table: &[f32]| table[step] + (table[next] - table[step]) * fraction;
    let (low, high) = (read(&self.min_area), read(&self.max_area));
    low + (high - low) * p_pick
  }
}

impl CurveRange {
  /// [`at`](CurveRange::at) for many things at once: the value at each age in `p_ages` for the pick at the same place in `p_picks`. For something that
  /// works out thousands of values a frame, and for a caller that is across a language boundary, where one call is far cheaper than thousands.
  ///
  /// # Panics
  /// When the two lists are not the same length.
  pub fn at_many(&self, p_ages: &[f32], p_picks: &[f32]) -> Vec<f32> {
    assert_eq!(p_ages.len(), p_picks.len(), "there is one pick for each age");
    p_ages.iter().zip(p_picks).map(|(age, pick)| self.at(*age, *pick)).collect()
  }

  /// [`area`](CurveRange::area) for many things at once, as [`at_many`](CurveRange::at_many) is for `at`.
  ///
  /// # Panics
  /// When the two lists are not the same length.
  pub fn area_many(&self, p_ages: &[f32], p_picks: &[f32]) -> Vec<f32> {
    assert_eq!(p_ages.len(), p_picks.len(), "there is one pick for each age");
    p_ages.iter().zip(p_picks).map(|(age, pick)| self.area(*age, *pick)).collect()
  }
}

impl From<f32> for CurveRange {
  fn from(p_value: f32) -> Self {
    CurveRange::from((p_value, p_value))
  }
}

impl From<(f32, f32)> for CurveRange {
  fn from((p_min, p_max): (f32, f32)) -> Self {
    CurveRange::new(Curve::new([(0.0, p_min)]), Curve::new([(0.0, p_max)]))
  }
}

impl From<Curve> for CurveRange {
  fn from(p_curve: Curve) -> Self {
    CurveRange::new(p_curve.clone(), p_curve)
  }
}

impl From<(Curve, Curve)> for CurveRange {
  fn from((p_min, p_max): (Curve, Curve)) -> Self {
    CurveRange::new(p_min, p_max)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_pick_chooses_a_place_between_the_curves() {
    let range = CurveRange::from((2.0, 6.0));
    assert_eq!(range.at(0.5, 0.0), 2.0);
    assert_eq!(range.at(0.5, 1.0), 6.0);
    assert_eq!(range.at(0.5, 0.25), 3.0);
  }

  #[test]
  fn the_area_of_a_flat_range_grows_with_age() {
    let range = CurveRange::from(4.0);
    assert!((range.area(0.5, 0.0) - 2.0).abs() < 1e-3);
    assert!((range.area(1.0, 1.0) - 4.0).abs() < 1e-3);
  }

  #[test]
  fn many_values_at_once_are_the_values_one_at_a_time() {
    let range = CurveRange::from((Curve::new([(0.0, 0.0), (255.0, 10.0)]), Curve::new([(0.0, 2.0), (255.0, 20.0)])));
    let (ages, picks) = ([0.0, 0.25, 0.5, 1.0], [0.0, 0.3, 0.7, 1.0]);
    let at: Vec<f32> = ages.iter().zip(picks).map(|(age, pick)| range.at(*age, pick)).collect();
    let area: Vec<f32> = ages.iter().zip(picks).map(|(age, pick)| range.area(*age, pick)).collect();
    assert_eq!(range.at_many(&ages, &picks), at);
    assert_eq!(range.area_many(&ages, &picks), area);
  }

  #[test]
  #[should_panic(expected = "one pick for each age")]
  fn lists_of_different_lengths_panic() {
    CurveRange::from(1.0).at_many(&[0.0, 0.5], &[0.0]);
  }

  #[test]
  fn zero_is_known_without_looking_at_a_pick() {
    assert!(CurveRange::from(0.0).is_zero());
    assert!(!CurveRange::from((0.0, 1.0)).is_zero());
  }
}
