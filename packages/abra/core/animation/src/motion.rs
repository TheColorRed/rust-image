use abra_core::{CurveRange, value_noise};

/// What a thing picked at random from each of the ranges of a [`Motion`], from 0.0 (the lower curve) to 1.0 (the upper), so no two things move alike.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoverPicks {
  pub velocity_x: f32,
  pub velocity_y: f32,
  pub orbital: f32,
  pub radial: f32,
  pub noise: f32,
}

/// One thing that moves: where it starts, the point it turns and moves away from, how fast it starts, and what it picked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mover {
  /// Where it is at birth.
  pub born: (f32, f32),
  /// The point orbital and radial velocity move it around and away from.
  pub center: (f32, f32),
  /// How far it travels over a whole life, in each direction, before anything else moves it.
  pub speed: (f32, f32),
  pub picks: MoverPicks,
}

/// How many numbers describe one [`Mover`] in [`Mover::from_values`]: `born` (x, y), `center` (x, y), `speed` (x, y), then the picks for the
/// velocity in x, the velocity in y, the orbital turn, the radial push and the noise.
pub const MOVER_VALUES: usize = 11;

impl Mover {
  /// A mover from [`MOVER_VALUES`] numbers.
  ///
  /// # Panics
  /// When there are not that many.
  pub fn from_values(p_values: &[f32]) -> Mover {
    let [born_x, born_y, center_x, center_y, speed_x, speed_y, velocity_x, velocity_y, orbital, radial, noise] = *p_values else {
      panic!("a mover is {MOVER_VALUES} numbers");
    };
    Mover {
      born: (born_x, born_y),
      center: (center_x, center_y),
      speed: (speed_x, speed_y),
      picks: MoverPicks { velocity_x, velocity_y, orbital, radial, noise },
    }
  }
}

/// How things move over a life, with an age from 0.0 (birth) to 1.0 (the end): pushed by a velocity that follows a curve, turned around a point,
/// moved toward or away from it, pulled down and moved about by smooth noise, in a form that works out many at once.
#[derive(Clone, Debug)]
pub struct Motion {
  /// The velocity in x and in y over a life, added up as a thing ages.
  velocity: (CurveRange, CurveRange),
  /// Degrees turned around the center, added up over a life.
  orbital: CurveRange,
  /// Pixels moved along the line from the center, added up over a life.
  radial: CurveRange,
  gravity: f32,
  /// How quickly the start speed is lost over a life: 0.0 keeps it, and the higher it is the sooner a thing slows to a stop.
  drag: f32,
  /// A constant push in x and y, in pixels over a whole life squared, besides gravity.
  wind: (f32, f32),
  /// How far noise moves a thing over a life, and the size of one swirl of it in pixels.
  noise: Option<(CurveRange, f32)>,
  /// How many layers of noise are added, each at twice the detail and half the strength of the one before.
  noise_octaves: u32,
  /// How far the noise slides across the picture over a whole life, in swirls in x and y.
  noise_scroll: (f32, f32),
  /// Whether the strength of the noise is a share of one swirl instead of pixels, so bigger swirls move things farther.
  noise_damping: bool,
  /// The box things bounce around in, as `(left, top, right, bottom)`, or nothing when they are free to leave.
  bounds: Option<(f32, f32, f32, f32)>,
  seed: u64,
}

impl Default for Motion {
  /// Things that only travel at their start speed.
  fn default() -> Self {
    Motion {
      velocity: (0.0.into(), 0.0.into()),
      orbital: 0.0.into(),
      radial: 0.0.into(),
      gravity: 0.0,
      drag: 0.0,
      wind: (0.0, 0.0),
      noise: None,
      noise_octaves: 1,
      noise_scroll: (0.0, 0.0),
      noise_damping: false,
      bounds: None,
      seed: 0,
    }
  }
}

impl Motion {
  pub fn new() -> Self {
    Motion::default()
  }

  /// The velocity in pixels in x and in y over a life.
  pub fn with_velocity(mut self, p_x: impl Into<CurveRange>, p_y: impl Into<CurveRange>) -> Self {
    self.velocity = (p_x.into(), p_y.into());
    self
  }

  /// Degrees turned around the center over a life.
  pub fn with_orbital(mut self, p_degrees: impl Into<CurveRange>) -> Self {
    self.orbital = p_degrees.into();
    self
  }

  /// Pixels moved along the line from the center over a life.
  pub fn with_radial(mut self, p_pixels: impl Into<CurveRange>) -> Self {
    self.radial = p_pixels.into();
    self
  }

  /// How strongly things are pulled down, in pixels over a whole life.
  pub fn with_gravity(mut self, p_gravity: f32) -> Self {
    self.gravity = p_gravity;
    self
  }

  /// How quickly the start speed is lost over a life. At 0.0 (the default) a thing keeps its speed; at 3.0 it has slowed to a crawl by the end. It
  /// does not slow the velocity over a life, only the speed a thing is born with.
  pub fn with_drag(mut self, p_drag: f32) -> Self {
    self.drag = p_drag.max(0.0);
    self
  }

  /// A constant push in x and in y, in pixels over a whole life squared (a wind that blows the whole life). Gravity is separate and also pulls.
  pub fn with_wind(mut self, p_x: f32, p_y: f32) -> Self {
    self.wind = (p_x, p_y);
    self
  }

  /// How many layers of noise are added (at least 1), each at twice the detail and half the strength of the one before: more layers give a rougher,
  /// more turbulent path.
  pub fn with_noise_octaves(mut self, p_octaves: u32) -> Self {
    self.noise_octaves = p_octaves.max(1);
    self
  }

  /// How far the noise slides across the picture over a whole life, in swirls in x and in y, so the push on a thing changes as it ages.
  pub fn with_noise_scroll(mut self, p_x: f32, p_y: f32) -> Self {
    self.noise_scroll = (p_x, p_y);
    self
  }

  /// Whether the strength of the noise is a share of one swirl in place of pixels. A strength of 1.0 then moves a thing one swirl, so swirls twice
  /// the size move things twice as far and the noise keeps its look when its size changes (Unity calls this damping).
  pub fn with_noise_damping(mut self, p_damping: bool) -> Self {
    self.noise_damping = p_damping;
    self
  }

  /// A box that things bounce around in, in pixels: `(left, top, right, bottom)`. A thing that would leave it bounces back, as off a wall, without
  /// losing speed. The path inside the box is the same as without it. Worked out from where the thing would have been, so there is no step by step.
  pub fn with_bounds(mut self, p_left: f32, p_top: f32, p_right: f32, p_bottom: f32) -> Self {
    self.bounds = Some((p_left.min(p_right), p_top.min(p_bottom), p_left.max(p_right), p_top.max(p_bottom)));
    self
  }

  /// The velocity of a mover at `p_age` as `(x, y)` in pixels over a whole life, found from where it is a moment before and a moment after.
  pub fn velocity_at(&self, p_mover: &Mover, p_age: f32) -> (f32, f32) {
    const STEP: f32 = 0.005;
    let (before, after) = ((p_age - STEP).max(0.0), (p_age + STEP).min(1.0));
    let (from, to) = (self.at(p_mover, before), self.at(p_mover, after));
    let span = (after - before).max(f32::EPSILON);
    ((to.0 - from.0) / span, (to.1 - from.1) / span)
  }

  /// How fast a mover is going at `p_age`, in pixels over a whole life.
  pub fn speed_at(&self, p_mover: &Mover, p_age: f32) -> f32 {
    let (x, y) = self.velocity_at(p_mover, p_age);
    x.hypot(y)
  }

  /// Smooth noise that moves things about: how far in pixels over a life, and the size of one swirl in pixels (at least 1). The `p_seed` gives the
  /// same noise each time.
  pub fn with_noise(mut self, p_strength: impl Into<CurveRange>, p_swirl_size: f32, p_seed: u64) -> Self {
    self.noise = Some((p_strength.into(), p_swirl_size.max(1.0)));
    self.seed = p_seed;
    self
  }

  /// Where a mover is at `p_age`.
  pub fn at(&self, p_mover: &Mover, p_age: f32) -> (f32, f32) {
    let (velocity_x, velocity_y) = &self.velocity;
    let picks = &p_mover.picks;
    // With drag the start speed carries a thing less far than it would: the distance of a speed that decays as it goes.
    let travelled = if self.drag > 1e-6 { (1.0 - (-self.drag * p_age).exp()) / self.drag } else { p_age };
    let mut x = p_mover.born.0 + p_mover.speed.0 * travelled + velocity_x.area(p_age, picks.velocity_x);
    let mut y = p_mover.born.1 + p_mover.speed.1 * travelled + velocity_y.area(p_age, picks.velocity_y);

    // Orbital velocity turns the thing around the center and radial velocity moves it along the line from it.
    let (dx, dy) = (x - p_mover.center.0, y - p_mover.center.1);
    let length = (dx * dx + dy * dy).sqrt();
    if length > 0.0 {
      let scale = (length + self.radial.area(p_age, picks.radial)).max(0.0) / length;
      let (sin, cos) = self.orbital.area(p_age, picks.orbital).to_radians().sin_cos();
      x = p_mover.center.0 + (dx * cos - dy * sin) * scale;
      y = p_mover.center.1 + (dx * sin + dy * cos) * scale;
    }

    // Gravity pulls the thing down after it has been pushed or turned, so that it does not change which way it is pushed.
    x += 0.5 * self.wind.0 * p_age * p_age;
    y += 0.5 * (self.gravity + self.wind.1) * p_age * p_age;

    // Noise is read where the thing is, so things near each other are pushed the same way. It grows with age so a thing is born where it was
    // meant to be.
    if let Some((strength, swirl)) = &self.noise {
      let amount = strength.at(p_age, picks.noise) * p_age * if self.noise_damping { *swirl } else { 1.0 };
      let (nx, ny) = (x / swirl + self.noise_scroll.0 * p_age, y / swirl + self.noise_scroll.1 * p_age);
      // Each layer is twice as fine and half as strong as the one before; with one layer this is plain noise.
      let (mut push_x, mut push_y, mut total) = (0.0, 0.0, 0.0);
      for octave in 0..self.noise_octaves {
        let (detail, weight) = (2.0f32.powi(octave as i32), 0.5f32.powi(octave as i32));
        let seed = self.seed.wrapping_add(octave as u64);
        push_x += weight * value_noise(nx * detail, ny * detail, seed);
        push_y += weight * value_noise(nx * detail + 31.7, ny * detail + 17.3, seed);
        total += weight;
      }
      x += amount * push_x / total;
      y += amount * push_y / total;
    }
    match self.bounds {
      Some((left, top, right, bottom)) => (Self::bounced(x, left, right), Self::bounced(y, top, bottom)),
      None => (x, y),
    }
  }

  /// `p_value` folded back into `p_low` to `p_high` as if it bounced off both ends.
  fn bounced(p_value: f32, p_low: f32, p_high: f32) -> f32 {
    let width = p_high - p_low;
    if width <= 0.0 {
      return p_low;
    }
    let along = (p_value - p_low).rem_euclid(2.0 * width);
    p_low + if along > width { 2.0 * width - along } else { along }
  }

  /// Where many movers are, in one call: for each place in `p_which` the mover at that index of `p_movers`, at the age at the same place in `p_ages`.
  /// The result is `x, y` for each, one after the other. A mover can be listed more than once, at different ages, which is how the history of a
  /// mover is worked out.
  ///
  /// # Panics
  /// When `p_which` and `p_ages` are not the same length, or an index is past the end of `p_movers`.
  pub fn at_many(&self, p_movers: &[Mover], p_which: &[u32], p_ages: &[f32]) -> Vec<f32> {
    assert_eq!(p_which.len(), p_ages.len(), "there is one age for each mover");
    let mut positions = Vec::with_capacity(p_which.len() * 2);
    for (index, age) in p_which.iter().zip(p_ages) {
      let (x, y) = self.at(&p_movers[*index as usize], *age);
      positions.push(x);
      positions.push(y);
    }
    positions
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Curve;

  fn mover(p_x: f32) -> Mover {
    Mover::from_values(&[p_x, 10.0, 50.0, 50.0, 4.0, -3.0, 0.2, 0.7, 0.4, 0.9, 0.5])
  }

  fn busy() -> Motion {
    Motion::new()
      .with_velocity((Curve::new([(0.0, 1.0), (255.0, 6.0)]), Curve::new([(0.0, 3.0), (255.0, 9.0)])), 2.0)
      .with_orbital((0.0, 90.0))
      .with_radial(Curve::new([(0.0, -5.0), (255.0, 20.0)]))
      .with_gravity(30.0)
      .with_noise((0.0, 12.0), 8.0, 7)
  }

  #[test]
  fn a_mover_with_no_motion_travels_at_its_start_speed() {
    let at = Motion::new().at(&Mover { born: (10.0, 20.0), speed: (4.0, -2.0), ..Mover::default() }, 0.5);
    assert_eq!(at, (12.0, 19.0));
  }

  #[test]
  fn a_mover_is_at_its_birth_point_at_age_zero() {
    let at = busy().at(&mover(30.0), 0.0);
    assert!((at.0 - 30.0).abs() < 1e-4 && (at.1 - 10.0).abs() < 1e-4, "{at:?}");
  }

  #[test]
  fn gravity_pulls_down_by_half_g_t_squared() {
    let at = Motion::new().with_gravity(8.0).at(&Mover::default(), 0.5);
    assert_eq!(at, (0.0, 1.0));
  }

  #[test]
  fn drag_carries_the_start_speed_less_far_and_none_is_unchanged() {
    let thing = Mover { speed: (100.0, 0.0), ..Mover::default() };
    assert_eq!(Motion::new().with_drag(0.0).at(&thing, 1.0), (100.0, 0.0));
    let slowed = Motion::new().with_drag(3.0).at(&thing, 1.0).0;
    assert!(slowed < 40.0 && slowed > 25.0, "{slowed}");
    // At the start the speed is the same, so the first moments are not slowed.
    assert!((Motion::new().with_drag(3.0).at(&thing, 0.01).0 - 1.0).abs() < 0.05);
  }

  #[test]
  fn wind_pushes_by_half_a_t_squared_in_each_direction() {
    let at = Motion::new().with_wind(8.0, -4.0).at(&Mover::default(), 0.5);
    assert_eq!(at, (1.0, -0.5));
  }

  #[test]
  fn more_noise_layers_and_a_scroll_change_the_path_and_one_layer_is_plain_noise() {
    let plain = Motion::new().with_noise((0.0, 12.0), 8.0, 7);
    let thing = mover(30.0);
    assert_eq!(plain.clone().with_noise_octaves(1).at(&thing, 0.8), plain.at(&thing, 0.8));
    assert_ne!(plain.clone().with_noise_octaves(3).at(&thing, 0.8), plain.at(&thing, 0.8));
    assert_ne!(plain.clone().with_noise_scroll(5.0, 0.0).at(&thing, 0.8), plain.at(&thing, 0.8));
    assert_eq!(plain.clone().with_noise_scroll(5.0, 3.0).at(&thing, 0.0), plain.at(&thing, 0.0));
  }

  #[test]
  fn damped_noise_moves_things_farther_when_the_swirls_are_bigger() {
    let thing = mover(30.0);
    let far = |swirl: f32, damped: bool| {
      let motion = Motion::new().with_noise(0.5, swirl, 7).with_noise_damping(damped);
      let (x, y) = motion.at(&thing, 1.0);
      let (px, py) = Motion::new().at(&thing, 1.0);
      (x - px).hypot(y - py)
    };
    assert!(far(40.0, true) > far(10.0, true) * 2.0, "{} {}", far(40.0, true), far(10.0, true));
    assert!((far(40.0, false) - far(10.0, false)).abs() < far(40.0, false), "undamped does not scale with the swirl");
  }

  #[test]
  fn a_thing_bounces_off_the_sides_of_its_bounds_and_stays_inside() {
    let motion = Motion::new().with_bounds(0.0, 0.0, 100.0, 100.0);
    let thing = Mover { born: (90.0, 50.0), speed: (40.0, 0.0), ..Mover::default() };
    assert_eq!(motion.at(&thing, 0.0), (90.0, 50.0));
    assert_eq!(motion.at(&thing, 0.5), (90.0, 50.0), "off the right wall at 100 and back, 10 + 10");
    assert_eq!(motion.at(&thing, 1.0), (70.0, 50.0));
    for step in 0..=20 {
      let (x, y) = motion.at(&Mover { born: (50.0, 50.0), speed: (900.0, -700.0), ..Mover::default() }, step as f32 / 20.0);
      assert!((0.0..=100.0).contains(&x) && (0.0..=100.0).contains(&y), "{x} {y}");
    }
  }

  #[test]
  fn speed_is_how_far_a_thing_goes_over_a_life_at_that_moment() {
    let thing = Mover { speed: (30.0, 40.0), ..Mover::default() };
    assert!((Motion::new().speed_at(&thing, 0.5) - 50.0).abs() < 0.01);
    let slowed = Motion::new().with_drag(4.0);
    assert!(slowed.speed_at(&thing, 0.9) < slowed.speed_at(&thing, 0.1));
  }

  #[test]
  fn velocity_points_the_way_a_thing_is_going() {
    let thing = Mover { speed: (30.0, -40.0), ..Mover::default() };
    let (x, y) = Motion::new().velocity_at(&thing, 0.5);
    assert!((x - 30.0).abs() < 0.01 && (y + 40.0).abs() < 0.01, "{x} {y}");
  }

  #[test]
  fn many_at_once_are_the_positions_one_at_a_time_and_a_mover_can_be_listed_again() {
    let motion = busy();
    let movers = [mover(5.0), mover(60.0)];
    let (which, ages) = ([0u32, 1, 1, 0], [0.1, 0.4, 0.9, 1.0]);
    let many = motion.at_many(&movers, &which, &ages);
    for (place, (index, age)) in which.iter().zip(ages).enumerate() {
      let (x, y) = motion.at(&movers[*index as usize], age);
      assert_eq!((many[place * 2], many[place * 2 + 1]), (x, y));
    }
  }

  #[test]
  #[should_panic(expected = "one age for each mover")]
  fn lists_of_different_lengths_panic() {
    Motion::new().at_many(&[Mover::default()], &[0, 0], &[0.0]);
  }

  #[test]
  #[should_panic(expected = "a mover is 11 numbers")]
  fn a_mover_of_the_wrong_number_of_values_panics() {
    Mover::from_values(&[1.0, 2.0]);
  }
}
