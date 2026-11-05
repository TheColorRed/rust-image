use abra_core::CurveRange;

/// How a thing is thrown when it is born: a speed picked from a range, in a direction within `spread` degrees either side of `angle` (0 is up, 90 is
/// right), or straight out from the point it moves around when it is set to go outward. The result is a [`Mover`](crate::Mover)'s start speed.
#[derive(Clone, Debug)]
pub struct Impulse {
  angle: f32,
  spread: f32,
  speed: CurveRange,
  outward: bool,
}

impl Impulse {
  /// An impulse toward `p_angle` degrees, give or take `p_spread` degrees, at a speed picked from `p_speed` in pixels over a whole life.
  pub fn new(p_angle: f32, p_spread: f32, p_speed: impl Into<CurveRange>) -> Self {
    Impulse { angle: p_angle, spread: p_spread.max(0.0), speed: p_speed.into(), outward: false }
  }

  /// Throws things away from the point they move around, in place of toward the angle. The spread is then either side of the line out from it.
  pub fn with_outward(mut self, p_outward: bool) -> Self {
    self.outward = p_outward;
    self
  }

  /// The start speed `(x, y)` of a thing born at `p_born`, thrown from `p_center`. `p_direction` and `p_speed` are its picks from 0.0 to 1.0: where in
  /// the spread it goes and where in the speed range its speed falls.
  pub fn velocity(&self, p_born: (f32, f32), p_center: (f32, f32), p_direction: f32, p_speed: f32) -> (f32, f32) {
    let base = if self.outward { (p_born.0 - p_center.0).atan2(p_center.1 - p_born.1).to_degrees() } else { self.angle };
    let (sin, cos) = (base + (p_direction * 2.0 - 1.0) * self.spread).to_radians().sin_cos();
    let speed = self.speed.at(0.0, p_speed);
    (sin * speed, -cos * speed)
  }

  /// [`velocity`](Impulse::velocity) for many things in one call. `p_born` and `p_center` are `x, y` for each thing, one after the other, and
  /// `p_directions` and `p_speeds` are a pick for each. The result is `x, y` for each.
  ///
  /// # Panics
  /// When the lists do not all describe the same number of things.
  pub fn velocity_many(&self, p_born: &[f32], p_center: &[f32], p_directions: &[f32], p_speeds: &[f32]) -> Vec<f32> {
    let count = p_directions.len();
    assert!(p_speeds.len() == count && p_born.len() == count * 2 && p_center.len() == count * 2, "there is a place, a center and two picks for each thing");
    let mut velocities = Vec::with_capacity(count * 2);
    for i in 0..count {
      let (x, y) = self.velocity((p_born[i * 2], p_born[i * 2 + 1]), (p_center[i * 2], p_center[i * 2 + 1]), p_directions[i], p_speeds[i]);
      velocities.push(x);
      velocities.push(y);
    }
    velocities
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_goes_up_at_angle_zero_and_right_at_ninety() {
    let (x, y) = Impulse::new(0.0, 0.0, 10.0).velocity((0.0, 0.0), (0.0, 0.0), 0.5, 0.5);
    assert!(x.abs() < 1e-4 && (y + 10.0).abs() < 1e-4, "{x} {y}");
    let (x, y) = Impulse::new(90.0, 0.0, 10.0).velocity((0.0, 0.0), (0.0, 0.0), 0.5, 0.5);
    assert!((x - 10.0).abs() < 1e-4 && y.abs() < 1e-4, "{x} {y}");
  }

  #[test]
  fn the_direction_pick_moves_it_across_the_spread() {
    let impulse = Impulse::new(0.0, 90.0, 10.0);
    let (left, _) = impulse.velocity((0.0, 0.0), (0.0, 0.0), 0.0, 0.5);
    let (right, _) = impulse.velocity((0.0, 0.0), (0.0, 0.0), 1.0, 0.5);
    assert!((left + 10.0).abs() < 1e-4 && (right - 10.0).abs() < 1e-4, "{left} {right}");
  }

  #[test]
  fn the_speed_pick_moves_it_across_the_speed_range() {
    let impulse = Impulse::new(90.0, 0.0, (2.0, 6.0));
    assert!((impulse.velocity((0.0, 0.0), (0.0, 0.0), 0.5, 0.0).0 - 2.0).abs() < 1e-4);
    assert!((impulse.velocity((0.0, 0.0), (0.0, 0.0), 0.5, 1.0).0 - 6.0).abs() < 1e-4);
  }

  #[test]
  fn many_at_once_are_the_velocities_one_at_a_time() {
    let impulse = Impulse::new(30.0, 45.0, (2.0, 6.0));
    let (born, center, directions, speeds) = ([1.0, 2.0, 30.0, 40.0], [5.0, 5.0, 0.0, 0.0], [0.1, 0.9], [0.3, 0.8]);
    let many = impulse.velocity_many(&born, &center, &directions, &speeds);
    for i in 0..2 {
      let (x, y) = impulse.velocity((born[i * 2], born[i * 2 + 1]), (center[i * 2], center[i * 2 + 1]), directions[i], speeds[i]);
      assert_eq!((many[i * 2], many[i * 2 + 1]), (x, y));
    }
  }

  #[test]
  #[should_panic(expected = "a place, a center and two picks")]
  fn lists_that_do_not_agree_panic() {
    Impulse::new(0.0, 0.0, 1.0).velocity_many(&[0.0, 0.0], &[0.0, 0.0], &[0.5, 0.5], &[0.5]);
  }

  #[test]
  fn outward_goes_away_from_the_center() {
    let impulse = Impulse::new(0.0, 0.0, 10.0).with_outward(true);
    let (x, y) = impulse.velocity((60.0, 50.0), (50.0, 50.0), 0.5, 0.5);
    assert!((x - 10.0).abs() < 1e-4 && y.abs() < 1e-4, "{x} {y}");
    let (x, y) = impulse.velocity((50.0, 40.0), (50.0, 50.0), 0.5, 0.5);
    assert!(x.abs() < 1e-4 && (y + 10.0).abs() < 1e-4, "{x} {y}");
  }
}
