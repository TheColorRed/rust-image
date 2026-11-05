//! Where many things are at a moment of their life (`animation`): `Motion`, `Movers`, `Impulse` and `Schedule`. A tuple of the library is an array here, and a list of
//! the wrong length panics. `usize` is a `u32`.
//!
//! What is not exported, and why:
//! - `Mover` and `MoverPicks` are plain data; a list of movers is one flat list of numbers (11 for each) in `Movers::new`, so thousands of them
//!   are made in one call and not as an object each.
//! - `Ensemble` and its `Frame` are not exported: they work out the numbers a part of the bindings turns into something (`Swarm` of the particle
//!   system), so thousands of them never cross as numbers.
//! - `Motion::at` (one mover) is not exported; across a language boundary a call for each is what `Motion::at_many` is for.

use std::sync::Arc;

use abra::animation::prelude as animation;

use crate::Curve;
use crate::sprites::CurveRange;

/// Many things that move, made once from a flat list of numbers and kept, so a frame only has to say how old they are. 11 numbers for each: where
/// it is born (`x`, `y`), the point it turns around and moves away from (`x`, `y`), its start speed (`x`, `y`), then its picks, from 0 to 1, for the
/// velocity in x, the velocity in y, the orbital turn, the radial push and the noise.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Movers(Vec<animation::Mover>);

#[uniffi::export]
impl Movers {
  /// Movers from a list that is a whole number of them (11 numbers each); any other length panics.
  #[uniffi::constructor]
  pub fn new(values: Vec<f32>) -> Self {
    assert!(values.len() % animation::MOVER_VALUES == 0, "a mover is {} numbers", animation::MOVER_VALUES);
    Self(values.chunks_exact(animation::MOVER_VALUES).map(animation::Mover::from_values).collect())
  }

  /// How many movers there are.
  pub fn len(&self) -> u32 {
    self.0.len() as u32
  }
}

/// How things move over a life, with an age from 0 (birth) to 1 (the end): a velocity that follows a curve, turning around a point, moving toward or
/// away from it, gravity and smooth noise.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Motion(animation::Motion);

impl Motion {
  /// The motion of the library, for other parts of the bindings that need one.
  pub(crate) fn core(&self) -> animation::Motion {
    self.0.clone()
  }
}

#[uniffi::export]
impl Motion {
  /// Things that only travel at their start speed.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(animation::Motion::new())
  }

  /// The velocity in pixels in x and in y over a life.
  pub fn with_velocity(&self, x: Arc<CurveRange>, y: Arc<CurveRange>) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_velocity(x.core(), y.core())))
  }

  /// Degrees turned around the center over a life.
  pub fn with_orbital(&self, degrees: Arc<CurveRange>) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_orbital(degrees.core())))
  }

  /// Pixels moved along the line from the center over a life.
  pub fn with_radial(&self, pixels: Arc<CurveRange>) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_radial(pixels.core())))
  }

  /// How strongly things are pulled down, in pixels over a whole life.
  pub fn with_gravity(&self, gravity: f32) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_gravity(gravity)))
  }

  /// How quickly the start speed is lost over a life. At 0 (the default) a thing keeps its speed; at 3 it has slowed to a crawl by the end.
  pub fn with_drag(&self, drag: f32) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_drag(drag)))
  }

  /// A constant push in x and in y, in pixels over a whole life squared. Gravity is separate and also pulls.
  pub fn with_wind(&self, x: f32, y: f32) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_wind(x, y)))
  }

  /// How many layers of noise are added (at least 1), each at twice the detail and half the strength of the one before.
  pub fn with_noise_octaves(&self, octaves: u32) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_noise_octaves(octaves)))
  }

  /// How far the noise slides across the picture over a whole life, in swirls in x and in y.
  pub fn with_noise_scroll(&self, x: f32, y: f32) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_noise_scroll(x, y)))
  }

  /// Whether the strength of the noise is a share of one swirl in place of pixels, so bigger swirls move things farther.
  pub fn with_noise_damping(&self, damping: bool) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_noise_damping(damping)))
  }

  /// A box that things bounce around in, in pixels. A thing that would leave it bounces back, as off a wall, without losing speed.
  pub fn with_bounds(&self, left: f32, top: f32, right: f32, bottom: f32) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_bounds(left, top, right, bottom)))
  }

  /// How fast many movers are going, in one call: for each place in `which`, the mover at that index of `movers` at the age at the same place in
  /// `ages`, in pixels over a whole life. Lists of different lengths, or an index past the end of `movers`, panic.
  pub fn speeds(&self, movers: Arc<Movers>, which: Vec<u32>, ages: Vec<f32>) -> Vec<f32> {
    assert_eq!(which.len(), ages.len(), "there is one age for each mover");
    which.iter().zip(&ages).map(|(index, age)| self.0.speed_at(&movers.0[*index as usize], *age)).collect()
  }

  /// Smooth noise that moves things about: how far over a life, the size of one swirl in pixels (at least 1) and the seed.
  pub fn with_noise(&self, strength: Arc<CurveRange>, swirl_size: f32, seed: u64) -> Arc<Motion> {
    Arc::new(Motion(self.0.clone().with_noise(strength.core(), swirl_size, seed)))
  }

  /// Where many movers are in one call: for each place in `which`, the mover at that index of `movers`, at the age at the same place in `ages`.
  /// The result is `x, y` for each, one after the other. A mover can be listed more than once, at different ages, which is how a trail is worked
  /// out. Lists of different lengths, or an index past the end of `movers`, panic.
  pub fn at_many(&self, movers: Arc<Movers>, which: Vec<u32>, ages: Vec<f32>) -> Vec<f32> {
    self.0.at_many(&movers.0, &which, &ages)
  }
}

/// How a thing is thrown when it is born: a speed picked from a range, in a direction within `spread` degrees either side of `angle` (0 is up, 90 is
/// right), or straight out from the point it moves around when it is set to go outward.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Impulse(animation::Impulse);

impl Impulse {
  pub fn core(&self) -> animation::Impulse {
    self.0.clone()
  }
}

#[uniffi::export]
impl Impulse {
  /// An impulse toward `angle` degrees, give or take `spread` degrees, at a speed picked from `speed` in pixels over a whole life.
  #[uniffi::constructor]
  pub fn new(angle: f32, spread: f32, speed: Arc<CurveRange>) -> Self {
    Self(animation::Impulse::new(angle, spread, speed.core()))
  }

  /// Throws things away from the point they move around, in place of toward the angle.
  pub fn with_outward(&self, outward: bool) -> Arc<Impulse> {
    Arc::new(Impulse(self.0.clone().with_outward(outward)))
  }

  /// The start speed `[x, y]` of a thing born at (`born_x`, `born_y`) and thrown from (`center_x`, `center_y`). `direction` and `speed` are its picks
  /// from 0 to 1: where in the spread it goes and where in the speed range its speed falls.
  pub fn velocity(&self, born_x: f32, born_y: f32, center_x: f32, center_y: f32, direction: f32, speed: f32) -> Vec<f32> {
    let (x, y) = self.0.velocity((born_x, born_y), (center_x, center_y), direction, speed);
    vec![x, y]
  }

  /// `velocity` for many things in one call. `born` and `center` are `x, y` for each thing, one after the other, and `directions` and `speeds` are a
  /// pick for each. The result is `x, y` for each. Lists that do not describe the same number of things panic.
  pub fn velocity_many(&self, born: Vec<f32>, center: Vec<f32>, directions: Vec<f32>, speeds: Vec<f32>) -> Vec<f32> {
    self.0.velocity_many(&born, &center, &directions, &speeds)
  }
}

/// When things are born and how long each lives, over time in seconds: a steady rate of things a second, bursts of many at once, a delay before the first,
/// and a lifetime picked from a range. Things are numbered: the bursts first, in the order they were added, then the steady stream.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Schedule(animation::Schedule);

impl Schedule {
  /// The schedule of the library, for other parts of the bindings that need one.
  pub(crate) fn core(&self) -> animation::Schedule {
    self.0.clone()
  }
}

#[uniffi::export]
impl Schedule {
  /// A system that runs for `duration` seconds, with nothing born yet and things that live one second.
  #[uniffi::constructor]
  pub fn new(duration: f32) -> Self {
    Self(animation::Schedule::new(duration))
  }

  /// Whether the system starts over when its duration is up, so things go on being born.
  pub fn with_looping(&self, looping: bool) -> Arc<Schedule> {
    Arc::new(Schedule(self.0.clone().with_looping(looping)))
  }

  /// How many things are born each second, evenly spread over the duration.
  pub fn with_rate(&self, rate: f32) -> Arc<Schedule> {
    Arc::new(Schedule(self.0.clone().with_rate(rate)))
  }

  /// `count` things born together `time` seconds in. Each call adds another burst.
  pub fn with_burst(&self, time: f32, count: u32) -> Arc<Schedule> {
    Arc::new(Schedule(self.0.clone().with_burst(time, count)))
  }

  /// Seconds before anything is born.
  pub fn with_delay(&self, delay: f32) -> Arc<Schedule> {
    Arc::new(Schedule(self.0.clone().with_delay(delay)))
  }

  /// How long each thing lives, picked between `min` and `max` seconds.
  pub fn with_lifetime(&self, min: f32, max: f32) -> Arc<Schedule> {
    Arc::new(Schedule(self.0.clone().with_lifetime(min, max)))
  }

  /// How many things there are in one run: the bursts, and the rate over the duration, rounded.
  pub fn count(&self) -> u32 {
    self.0.count() as u32
  }

  /// Seconds into a run that thing `index` is born, or nothing when there is no such thing.
  pub fn birth(&self, index: u32) -> Option<f32> {
    self.0.birth(index as usize)
  }

  /// How long a thing lives, in seconds, for its pick (0 for the shortest life, 1 for the longest).
  pub fn lifetime(&self, pick: f32) -> f32 {
    self.0.lifetime(pick)
  }

  /// How far through its life thing `index` is at `time` seconds, from 0 (born) to 1 (gone), or nothing when it is not alive then.
  pub fn age(&self, index: u32, time: f32, pick: f32) -> Option<f32> {
    self.0.age(index as usize, time, pick)
  }
}

/// How the size, rotation and color of a thing follow how fast it is going, from `low` to `high` (pixels over a whole life). Each part is read at how far
/// the speed is between the two, from 0 at `low` to 1 at `high`.
#[derive(Clone, Debug, uniffi::Object)]
pub struct SpeedResponse(animation::SpeedResponse);

impl SpeedResponse {
  pub fn core(&self) -> animation::SpeedResponse {
    self.0.clone()
  }
}

#[uniffi::export]
impl SpeedResponse {
  /// A response to speeds from `low` to `high`, which does nothing until a part is added.
  #[uniffi::constructor]
  pub fn new(low: f32, high: f32) -> Self {
    Self(animation::SpeedResponse::new(low, high))
  }

  /// What the size is multiplied by, as a curve over the speed (its value at 0 to 255 along it).
  pub fn with_size(&self, curve: Arc<Curve>) -> Arc<SpeedResponse> {
    Arc::new(SpeedResponse(self.0.clone().with_size(curve.core())))
  }

  /// The degrees added to the rotation, as a curve over the speed.
  pub fn with_rotation(&self, curve: Arc<Curve>) -> Arc<SpeedResponse> {
    Arc::new(SpeedResponse(self.0.clone().with_rotation(curve.core())))
  }

  /// The color the thing is multiplied by over the speed: 257 colors of four numbers each (`red`, `green`, `blue`, `alpha`, 0 to 255). Any other length
  /// panics.
  pub fn with_colors(&self, colors: Vec<f32>) -> Arc<SpeedResponse> {
    Arc::new(SpeedResponse(self.0.clone().with_colors(&colors)))
  }
}

/// Noise that changes the size and the rotation of things, read where each is.
#[derive(Clone, Copy, Debug, uniffi::Object)]
pub struct PoseNoise(animation::PoseNoise);

impl PoseNoise {
  pub fn core(&self) -> animation::PoseNoise {
    self.0
  }
}

#[uniffi::export]
impl PoseNoise {
  /// The size changes by up to `size` of itself (0.5 is half again or half as much), the rotation by up to `rotation` degrees, over swirls `swirl` pixels
  /// across. The same `seed` gives the same noise.
  #[uniffi::constructor]
  pub fn new(size: f32, rotation: f32, swirl: f32, seed: u64) -> Self {
    Self(animation::PoseNoise::new(size, rotation, swirl, seed))
  }
}
