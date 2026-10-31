use std::ops::Deref;

use crate::{Path, PointF, Segment};

#[derive(Clone, Debug)]
/// A curve is a smooth line through points that gives one `y` for every `x`, such as a tone curve that maps an input
/// value to an output value.
/// A curve is an open [`Path`] that never goes back on itself in `x`. Use a Path for a line that can.
///
/// A curve derefs to its [`Path`], so every read-only path method (`flatten`, `bounds`, `length`, ...) can be called on
/// it directly. It does not deref mutably, because adding a segment could make it go back in `x`. Change it with
/// [`Curve::with_point`] instead.
pub struct Curve {
  path: Path,
}

impl Curve {
  /// Starts slow and ends fast, below the straight line the whole way, from `(0, 0)` to `(255, 255)`. It is `x ^ p_power`
  /// on a scale of `0` to `1`, so a power of `2` goes through `(128, 64)` and `3` through `(128, 32)`.
  /// - `p_power`: How strongly it eases. `1` is the straight line, and it should be above `0`.
  pub fn ease_in(p_power: f32) -> Curve {
    Curve::sampled(|p_t| p_t.powf(p_power))
  }

  /// Starts fast and ends slow, above the straight line the whole way, from `(0, 0)` to `(255, 255)`. It is the mirror of
  /// [`Curve::ease_in`], so a power of `2` goes through `(128, 191)`.
  /// - `p_power`: How strongly it eases. `1` is the straight line, and it should be above `0`.
  pub fn ease_out(p_power: f32) -> Curve {
    Curve::sampled(|p_t| 1.0 - (1.0 - p_t).powf(p_power))
  }

  /// Slow at both ends and fast in the middle, an "S": it eases in over the first half and out over the second, and goes
  /// through `(128, 128)`. A power of `2` goes through `(64, 32)` and `(192, 224)`.
  /// - `p_power`: How strongly it eases. `1` is the straight line, and it should be above `0`.
  pub fn ease_in_out(p_power: f32) -> Curve {
    Curve::sampled(|p_t| {
      if p_t < 0.5 { 0.5 * (2.0 * p_t).powf(p_power) } else { 1.0 - 0.5 * (2.0 * (1.0 - p_t)).powf(p_power) }
    })
  }

  /// Fast at both ends and slow in the middle, the inverse "S": it eases out over the first half and in over the second,
  /// and goes through `(128, 128)`. A power of `2` goes through `(64, 96)` and `(192, 160)`, and `3` through `(64, 112)`.
  /// - `p_power`: How strongly it eases. `1` is the straight line, and it should be above `0`.
  pub fn ease_out_in(p_power: f32) -> Curve {
    Curve::sampled(|p_t| {
      if p_t < 0.5 { 0.5 * (1.0 - (1.0 - 2.0 * p_t).powf(p_power)) } else { 0.5 + 0.5 * (2.0 * p_t - 1.0).powf(p_power) }
    })
  }

  /// An "S" that is flat at both ends, `3t² - 2t³` on a scale of `0` to `1`, from `(0, 0)` to `(255, 255)`. It goes through
  /// `(128, 128)` and, like [`Curve::ease_in_out`] with a power of `2`, is below the straight line first and above it
  /// after, but it is smooth where the two halves meet.
  pub fn smoothstep() -> Curve {
    Curve::sampled(|p_t| p_t * p_t * (3.0 - 2.0 * p_t))
  }

  /// A gentle [`Curve::ease_in`] that follows a sine wave, from `(0, 0)` to `(255, 255)`. It goes through about `(128, 75)`.
  pub fn ease_in_sine() -> Curve {
    Curve::sampled(|p_t| 1.0 - (p_t * std::f32::consts::FRAC_PI_2).cos())
  }

  /// A gentle [`Curve::ease_out`] that follows a sine wave, from `(0, 0)` to `(255, 255)`. It goes through about `(128, 180)`.
  pub fn ease_out_sine() -> Curve {
    Curve::sampled(|p_t| (p_t * std::f32::consts::FRAC_PI_2).sin())
  }

  /// A gentle [`Curve::ease_in_out`] that follows a sine wave, from `(0, 0)` to `(255, 255)`. It goes through `(128, 128)`.
  pub fn ease_in_out_sine() -> Curve {
    Curve::sampled(|p_t| (1.0 - (p_t * std::f32::consts::PI).cos()) / 2.0)
  }

  /// A curve shaped by two handles, as in the CSS `cubic-bezier()` function, from `(0, 0)` to `(255, 255)`. The handles
  /// are on a scale of `0` to `1`, so `(0.42, 0, 0.58, 1)` is CSS's `ease-in-out` and `(0, 0, 1, 1)` is the straight line.
  /// Returns `None` when a handle's `x` is outside `0..=1` or not a number, because the line would then go back in `x`.
  /// - `p_x1`: The `x` of the first handle, which pulls on the start.
  /// - `p_y1`: The `y` of the first handle.
  /// - `p_x2`: The `x` of the second handle, which pulls on the end.
  /// - `p_y2`: The `y` of the second handle.
  pub fn cubic_bezier(p_x1: f32, p_y1: f32, p_x2: f32, p_y2: f32) -> Option<Curve> {
    let mut path = Path::new();
    path.move_to((0.0, 0.0)).cubic_to((255.0 * p_x1, 255.0 * p_y1), (255.0 * p_x2, 255.0 * p_y2), (255.0, 255.0));
    if !(0.0..=1.0).contains(&p_x1) || !(0.0..=1.0).contains(&p_x2) {
      return None;
    }
    Curve::from_path(path)
  }

  /// A curve through points of `p_shape` taken evenly across `0` to `255`, where the shape maps `0..=1` to `0..=1`.
  fn sampled(p_shape: impl Fn(f32) -> f32) -> Curve {
    const STEPS: u32 = 16;
    Curve::new((0..=STEPS).map(|step| {
      let t = step as f32 / STEPS as f32;
      (255.0 * t, 255.0 * p_shape(t))
    }))
  }

  /// The curve flipped top to bottom on a scale of `0` to `255`: every `y` becomes `255 - y`, handles included. Flipping
  /// [`Curve::identity`] gives the line from `(0, 255)` to `(255, 0)`, which is a photographic negative, and flipping twice
  /// gives the curve back.
  pub fn inverted(&self) -> Curve {
    let flip = |p_point: PointF| PointF::new(p_point.x, 255.0 - p_point.y);
    let mut path = Path::new();
    path.move_to(flip(self.path.start()));
    for segment in self.path.segments() {
      match segment {
        Segment::Line { to } => path.line_to(flip(*to)),
        Segment::Quadratic { ctrl, to } => path.quad_to(flip(*ctrl), flip(*to)),
        Segment::Cubic { ctrl1, ctrl2, to } => path.cubic_to(flip(*ctrl1), flip(*ctrl2), flip(*to)),
      };
    }
    Curve { path }
  }

  /// The curve that changes nothing on a scale of `0` to `255`: a straight line from `(0, 0)` to `(255, 255)`.
  pub fn identity() -> Curve {
    Curve {
      path: Path::line((0.0, 0.0), (255.0, 255.0)),
    }
  }

  /// A straight line from `p_start` at `x` of `0` to `p_end` at `x` of `255`. The values are not limited to `0..=255`,
  /// so it can be any quantity that changes steadily across the range, such as a size that shrinks, and a start equal to
  /// the end gives a level line.
  /// - `p_start`: The `y` at the start.
  /// - `p_end`: The `y` at the end.
  pub fn from_to(p_start: f32, p_end: f32) -> Curve {
    Curve {
      path: Path::line((0.0, p_start), (255.0, p_end)),
    }
  }

  /// A cosine wave across `x` of `0` to `255`: it starts at its peak, `p_amplitude`, and falls to `-p_amplitude` and
  /// back up `p_cycles` times. The values are not limited to `0..=255`. Used as a velocity, one cycle adds up to an
  /// "S": the thing it moves swings one way, back across where it started and out the other way.
  /// - `p_amplitude`: How high and how low the wave goes, around `0`.
  /// - `p_cycles`: How many full waves fit across the range. It should be above `0`.
  pub fn cosine(p_amplitude: f32, p_cycles: f32) -> Curve {
    // Four points per half wave keep the line smooth and put the peaks and the zero crossings on a point.
    let points = ((p_cycles.max(0.0) * 4.0).ceil() as u32).max(1);
    Curve::new((0..=points).map(|step| {
      let t = step as f32 / points as f32;
      (255.0 * t, p_amplitude * (std::f32::consts::TAU * p_cycles * t).cos())
    }))
  }

  /// Creates a smooth curve that goes through every point.
  ///
  /// The points can be in any order. A point that shares its `x` with another replaces it, the last one given winning,
  /// and a point that is not a finite number is left out. Between the points the curve is a monotone cubic: it never
  /// goes above the higher of two neighbouring points or below the lower one, so a curve through rising points only
  /// rises and does not overshoot. Beyond the first and last points it stays level. No points give [`Curve::identity`],
  /// and one point gives a level line.
  /// - `p_points`: The points `(x, y)` the curve goes through.
  pub fn new(p_points: impl IntoIterator<Item = impl Into<PointF>>) -> Curve {
    let mut sorted: Vec<PointF> =
      p_points.into_iter().map(Into::into).filter(|point| point.x.is_finite() && point.y.is_finite()).collect();
    // The sort is stable, so of two points with the same x the one given last stays last.
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut points: Vec<PointF> = Vec::with_capacity(sorted.len());
    for point in sorted {
      match points.last_mut() {
        Some(last) if last.x == point.x => *last = point,
        _ => points.push(point),
      }
    }
    if points.is_empty() {
      return Curve::identity();
    }

    let mut path = Path::new();
    path.move_to(points[0]);
    if points.len() > 1 {
      let tangents = tangents(&points);
      for (index, pair) in points.windows(2).enumerate() {
        let (from, to) = (pair[0], pair[1]);
        // The handles sit a third of the way across, so `x` rises evenly along the segment.
        let third = (to.x - from.x) / 3.0;
        path.cubic_to(
          (from.x + third, from.y + tangents[index] * third),
          (to.x - third, to.y - tangents[index + 1] * third),
          to,
        );
      }
    }
    Curve { path }
  }

  /// Makes a curve from a path, or `None` when the path goes back in `x` and so has no single `y` for an `x`.
  /// - `p_path`: The path, whose points and control points must never move left.
  pub fn from_path(p_path: Path) -> Option<Curve> {
    let mut points = vec![p_path.start()];
    for segment in p_path.segments() {
      match segment {
        Segment::Line { to } => points.push(*to),
        Segment::Quadratic { ctrl, to } => points.extend([*ctrl, *to]),
        Segment::Cubic { ctrl1, ctrl2, to } => points.extend([*ctrl1, *ctrl2, *to]),
      }
    }
    let ordered = points.iter().all(|point| point.x.is_finite() && point.y.is_finite())
      && points.windows(2).all(|pair| pair[0].x <= pair[1].x);
    ordered.then_some(Curve { path: p_path })
  }

  /// Returns the curve with a point added, or moved if the curve already has a point at that `x`. The curve is smoothed
  /// through its points again, as in [`Curve::new`].
  /// - `p_x`: The `x` of the point.
  /// - `p_y`: The `y` of the point.
  pub fn with_point(self, p_x: f32, p_y: f32) -> Curve {
    let mut points = self.path.points();
    points.push(PointF::new(p_x, p_y));
    Curve::new(points)
  }

  /// The `y` of the curve at `p_x`. Before the first point and after the last it is the `y` of that point.
  /// - `p_x`: The `x` to read the curve at.
  pub fn value(&self, p_x: f32) -> f32 {
    let start = self.path.start();
    if !(p_x > start.x) {
      return start.y;
    }
    for (index, segment) in self.path.segments().iter().enumerate() {
      if p_x <= segment.end().x {
        // `x` only rises along a segment, so the position where it equals `p_x` can be halved in on.
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        for _ in 0..24 {
          let middle = (low + high) / 2.0;
          if self.path.point_at_segment(index, middle).x < p_x {
            low = middle;
          } else {
            high = middle;
          }
        }
        return self.path.point_at_segment(index, (low + high) / 2.0).y;
      }
    }
    self.path.end().y
  }

  /// The curve as a lookup table for the values `0` to `255`: the entry at each position is the curve's `y` there,
  /// rounded and held to `0..=255`.
  pub fn to_table(&self) -> [u8; 256] {
    std::array::from_fn(|index| self.value(index as f32).round().clamp(0.0, 255.0) as u8)
  }

  /// The path the curve is made of.
  pub fn as_path(&self) -> &Path {
    &self.path
  }
}

impl Default for Curve {
  fn default() -> Self {
    Curve::identity()
  }
}

impl Deref for Curve {
  type Target = Path;
  fn deref(&self) -> &Path {
    &self.path
  }
}

impl From<Curve> for Path {
  fn from(p_curve: Curve) -> Self {
    p_curve.path
  }
}

/// The slope the curve has at each point. Where the points stop rising or stop falling the slope is level, and elsewhere
/// it is the weighted harmonic mean of the slopes either side, which keeps the curve from going past its points.
fn tangents(p_points: &[PointF]) -> Vec<f32> {
  let widths: Vec<f32> = p_points.windows(2).map(|pair| pair[1].x - pair[0].x).collect();
  let slopes: Vec<f32> = p_points.windows(2).map(|pair| (pair[1].y - pair[0].y) / (pair[1].x - pair[0].x)).collect();
  let mut tangents = vec![0.0; p_points.len()];
  tangents[0] = slopes[0];
  tangents[p_points.len() - 1] = slopes[slopes.len() - 1];
  for index in 1..p_points.len() - 1 {
    let (before, after) = (slopes[index - 1], slopes[index]);
    if before * after > 0.0 {
      let (width_before, width_after) = (widths[index - 1], widths[index]);
      tangents[index] = 3.0 * (width_before + width_after)
        / ((2.0 * width_after + width_before) / before + (width_after + 2.0 * width_before) / after);
    }
  }
  tangents
}

#[cfg(test)]
mod tests {
  use super::*;

  fn close(p_a: f32, p_b: f32) -> bool {
    (p_a - p_b).abs() < 0.01
  }

  #[test]
  fn no_points_is_the_curve_that_changes_nothing() {
    let table = Curve::new(Vec::<(f32, f32)>::new()).to_table();
    assert!((0..256).all(|index| table[index] == index as u8));
  }

  #[test]
  fn from_to_is_a_straight_line_between_the_values() {
    let curve = Curve::from_to(10.0, 70.0);
    assert!(close(curve.value(0.0), 10.0));
    assert!(close(curve.value(127.5), 40.0));
    assert!(close(curve.value(255.0), 70.0));
    assert!(close(Curve::from_to(0.0, 255.0).value(100.0), Curve::identity().value(100.0)));
  }

  #[test]
  fn a_cosine_wave_peaks_crosses_zero_and_falls() {
    let curve = Curve::cosine(100.0, 1.0);
    assert!(close(curve.value(0.0), 100.0));
    assert!(close(curve.value(63.75), 0.0));
    assert!(close(curve.value(127.5), -100.0));
    assert!(close(curve.value(191.25), 0.0));
    assert!(close(curve.value(255.0), 100.0));
  }

  #[test]
  fn one_point_is_a_level_line() {
    let curve = Curve::new([(100.0, 60.0)]);
    assert!(close(curve.value(0.0), 60.0));
    assert!(close(curve.value(100.0), 60.0));
    assert!(close(curve.value(255.0), 60.0));
  }

  #[test]
  fn the_curve_goes_through_its_points() {
    let points = [(0.0, 10.0), (64.0, 40.0), (192.0, 215.0), (255.0, 250.0)];
    let curve = Curve::new(points);
    for (x, y) in points {
      assert!(close(curve.value(x), y), "at {x} the curve is {} and not {y}", curve.value(x));
    }
  }

  #[test]
  fn points_in_any_order_and_repeats_are_tidied() {
    let curve = Curve::new([(255.0, 255.0), (0.0, 0.0), (128.0, 10.0), (128.0, 90.0)]);
    assert!(close(curve.value(128.0), 90.0), "the last point at an x wins");
    assert_eq!(curve.points().len(), 3);
  }

  #[test]
  fn beyond_the_end_points_the_curve_stays_level() {
    let curve = Curve::new([(50.0, 20.0), (200.0, 180.0)]);
    assert!(close(curve.value(0.0), 20.0));
    assert!(close(curve.value(255.0), 180.0));
    assert!(close(curve.value(f32::NAN), 20.0));
  }

  #[test]
  fn rising_points_give_a_curve_that_only_rises() {
    let table = Curve::new([(0.0, 0.0), (64.0, 40.0), (192.0, 215.0), (255.0, 255.0)]).to_table();
    assert!(table.windows(2).all(|pair| pair[0] <= pair[1]), "an S-curve never goes back down");
  }

  #[test]
  fn the_curve_does_not_go_past_a_peak() {
    let curve = Curve::new([(0.0, 0.0), (100.0, 200.0), (200.0, 100.0), (255.0, 255.0)]);
    assert!((0..=255).all(|x| curve.value(x as f32) <= 255.01 && curve.value(x as f32) >= -0.01));
    assert!((0..=100).all(|x| curve.value(x as f32) <= 200.01), "no overshoot above the peak");
    assert!((100..=200).all(|x| curve.value(x as f32) >= 99.99), "no overshoot below the dip");
  }

  #[test]
  fn a_curve_can_be_made_from_a_path_that_does_not_go_back() {
    let mut path = Path::new();
    path.move_to((0.0, 0.0)).line_to((100.0, 50.0)).line_to((255.0, 255.0));
    let curve = Curve::from_path(path).expect("a path that only moves right");
    assert!(close(curve.value(50.0), 25.0));
  }

  #[test]
  fn a_path_that_goes_back_is_not_a_curve() {
    let mut path = Path::new();
    path.move_to((0.0, 0.0)).line_to((100.0, 50.0)).line_to((50.0, 80.0));
    assert!(Curve::from_path(path).is_none());
  }

  #[test]
  fn a_point_is_added_or_moved_with_a_point() {
    let curve = Curve::identity().with_point(128.0, 60.0);
    assert!(close(curve.value(128.0), 60.0));
    assert!(close(curve.value(0.0), 0.0) && close(curve.value(255.0), 255.0));
    let moved = curve.with_point(128.0, 100.0);
    assert!(close(moved.value(128.0), 100.0));
    assert_eq!(moved.points().len(), 3);
  }

  /// Within a level and a half, to allow for the curve being sampled between whole numbers.
  fn near(p_a: f32, p_b: f32) -> bool {
    (p_a - p_b).abs() < 1.5
  }

  #[test]
  fn every_ease_runs_from_corner_to_corner_and_only_rises() {
    for curve in [Curve::ease_in(2.0), Curve::ease_out(2.0), Curve::ease_in_out(2.0), Curve::ease_out_in(3.0)] {
      assert!(near(curve.value(0.0), 0.0) && near(curve.value(255.0), 255.0));
      assert!(curve.to_table().windows(2).all(|pair| pair[0] <= pair[1]));
    }
  }

  #[test]
  fn a_power_of_one_is_the_straight_line() {
    for curve in [Curve::ease_in(1.0), Curve::ease_out(1.0), Curve::ease_in_out(1.0), Curve::ease_out_in(1.0)] {
      let table = curve.to_table();
      assert!((0..256).all(|index| table[index] == index as u8));
    }
  }

  #[test]
  fn ease_in_starts_slow_and_ease_out_starts_fast() {
    let (slow, fast) = (Curve::ease_in(2.0), Curve::ease_out(2.0));
    assert!(near(slow.value(128.0), 64.0) && near(fast.value(128.0), 191.0));
    assert!(near(Curve::ease_in(3.0).value(128.0), 32.0));
  }

  #[test]
  fn ease_in_out_is_the_s_below_the_line_first_and_above_it_after() {
    let curve = Curve::ease_in_out(2.0);
    assert!(near(curve.value(64.0), 32.0) && near(curve.value(192.0), 224.0));
    assert!(near(curve.value(128.0), 128.0), "the middle stays where it is");
  }

  #[test]
  fn ease_out_in_is_the_inverse_s_above_the_line_first_and_below_it_after() {
    let curve = Curve::ease_out_in(2.0);
    assert!(near(curve.value(64.0), 96.0) && near(curve.value(192.0), 160.0));
    assert!(near(Curve::ease_out_in(3.0).value(64.0), 112.0));
    assert!(near(curve.value(128.0), 128.0));
  }

  #[test]
  fn smoothstep_and_the_sine_eases_have_their_known_shapes() {
    assert!(near(Curve::smoothstep().value(64.0), 40.0) && near(Curve::smoothstep().value(128.0), 128.0));
    assert!(near(Curve::ease_in_sine().value(128.0), 75.0));
    assert!(near(Curve::ease_out_sine().value(128.0), 180.0));
    assert!(near(Curve::ease_in_out_sine().value(128.0), 128.0) && near(Curve::ease_in_out_sine().value(64.0), 37.0));
    for curve in [Curve::smoothstep(), Curve::ease_in_sine(), Curve::ease_out_sine(), Curve::ease_in_out_sine()] {
      assert!(near(curve.value(0.0), 0.0) && near(curve.value(255.0), 255.0));
      assert!(curve.to_table().windows(2).all(|pair| pair[0] <= pair[1]));
    }
  }

  #[test]
  fn a_cubic_bezier_follows_its_handles() {
    let straight = Curve::cubic_bezier(0.0, 0.0, 1.0, 1.0).expect("handles inside the box").to_table();
    assert!((0..256).all(|index| (straight[index] as i32 - index as i32).abs() <= 1));
    let ease_in_out = Curve::cubic_bezier(0.42, 0.0, 0.58, 1.0).expect("handles inside the box");
    assert!(ease_in_out.value(64.0) < 64.0 && ease_in_out.value(192.0) > 192.0);
    let ease = Curve::cubic_bezier(0.25, 0.1, 0.25, 1.0).expect("handles inside the box");
    assert!(ease.value(128.0) > 128.0, "the CSS ease starts fast");
  }

  #[test]
  fn a_cubic_bezier_with_a_handle_outside_the_box_is_not_a_curve() {
    assert!(Curve::cubic_bezier(-0.1, 0.0, 0.5, 1.0).is_none());
    assert!(Curve::cubic_bezier(0.5, 0.0, 1.5, 1.0).is_none());
    assert!(Curve::cubic_bezier(f32::NAN, 0.0, 0.5, 1.0).is_none());
  }

  #[test]
  fn a_curve_that_falls_is_a_negative_and_can_be_made_by_points_or_by_flipping() {
    let by_points = Curve::new([(0.0, 255.0), (255.0, 0.0)]).to_table();
    let by_flipping = Curve::identity().inverted().to_table();
    assert_eq!(by_points, by_flipping);
    assert!((0..256).all(|index| by_points[index] == 255 - index as u8));
  }

  #[test]
  fn flipping_twice_gives_the_curve_back_and_flips_every_shape() {
    let curve = Curve::ease_in_out(2.0);
    assert_eq!(curve.inverted().inverted().to_table(), curve.to_table());
    assert!(near(curve.inverted().value(64.0), 255.0 - 32.0));
    let bezier = Curve::cubic_bezier(0.42, 0.0, 0.58, 1.0).expect("handles inside the box");
    assert!(near(bezier.inverted().value(100.0), 255.0 - bezier.value(100.0)));
  }

  #[test]
  fn a_curve_that_rises_and_falls_goes_up_to_its_peak_and_no_further() {
    let curve = Curve::new([(0.0, 0.0), (128.0, 255.0), (255.0, 0.0)]);
    assert!(near(curve.value(128.0), 255.0));
    assert!((0..=255).all(|x| curve.value(x as f32) <= 255.01 && curve.value(x as f32) >= -0.01));
  }

  #[test]
  fn the_table_rounds_and_holds_to_a_byte() {
    let table = Curve::new([(0.0, -40.0), (255.0, 300.0)]).to_table();
    assert_eq!(table[0], 0);
    assert_eq!(table[255], 255);
  }
}
