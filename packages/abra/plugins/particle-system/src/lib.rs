use abra::abra_core::image::gpu::{GpuPass, GpuProcessor};
use abra::abra_core::{CurveRange, Random};
use abra::animation::prelude::{Impulse, Motion, Mover, MoverPicks, PoseNoise, Schedule, SpeedResponse};
use abra::drawing::prelude::{Blend, SPRITE_MAX_WORDS, SPRITE_WORDS, Sprite, SpriteTrail, TRAIL_POINT_WORDS, TRAIL_WORDS, TrailPoint, sprites};
use abra::options::prelude::{Effect, Options};
use abra::prelude::*;

mod plan;
mod swarm;

pub use swarm::{PARTICLE_VALUES, Swarm, SwarmStyle};

pub mod prelude {
  pub use super::{Emitter, PARTICLE_VALUES, ParticleSystem, Swarm, SwarmStyle, particle_system};
  pub use abra::animation::prelude::{Impulse, PoseNoise, Schedule, SpeedResponse};
  pub use abra::drawing::prelude::Blend;
  pub use abra::options::prelude::Effect;
}

/// Where particles are born, in pixels of the image.
#[derive(Clone, Debug)]
pub enum Emitter {
  /// Anywhere on the image.
  Image,
  /// Inside a circle.
  Circle { center: (f32, f32), radius: f32 },
  /// Inside a rectangle: `(left, top, width, height)`.
  Rect(f32, f32, f32, f32),
  /// On a ring between `inner_radius` and `radius`.
  Ring { center: (f32, f32), radius: f32, inner_radius: f32 },
  /// Along a line from one point to another.
  Line((f32, f32), (f32, f32)),
  /// Anywhere on the image where the pixel is at least this light (0.0 to 1.0), so particles come from the bright
  /// parts of the picture. If no pixel is that light, they are born anywhere.
  Luminance(f32),
}

impl Emitter {
  /// The middle of the emitter, which radial and orbital velocity move around by default.
  fn center(&self, p_width: u32, p_height: u32) -> (f32, f32) {
    match self {
      Emitter::Image | Emitter::Luminance(_) => (p_width as f32 / 2.0, p_height as f32 / 2.0),
      Emitter::Circle { center, .. } | Emitter::Ring { center, .. } => *center,
      Emitter::Rect(left, top, width, height) => (left + width / 2.0, top + height / 2.0),
      Emitter::Line(from, to) => ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0),
    }
  }
}

/// Scatters particles over one image. There is no loop: every value comes from the app,
/// and `progress` picks the moment of each particle's life that is drawn. Each particle picks its own
/// values from the ranges, so no two are alike.
#[derive(Clone, Debug)]
pub struct ParticleSystem {
  emitter: Emitter,
  seed: u64,
  count: usize,
  /// Particle size in pixels over its life.
  size: CurveRange,
  /// Each particle starts with the color at a random spot on this gradient. With no start color and no color over
  /// lifetime, a particle takes the color of the pixel it was born on.
  start_color: Option<Gradient>,
  /// The gradient a particle follows from birth (time 0) to the end of its life (time 1). It multiplies the start
  /// color, and a color's alpha multiplies the opacity.
  color_over_lifetime: Option<Gradient>,
  /// The picture each particle is drawn as, scaled so its longer side is twice the particle's size. `None` draws a soft
  /// round dot. A particle's color tints the texture, and with no color set the texture is drawn as it is.
  texture: Option<Image>,
  /// Rotation of each particle in degrees over its life; the particle turns around its middle.
  rotation: CurveRange,
  /// How far back along its path a particle leaves a streak, as a share of its life (0.0 for none). The streak is drawn
  /// with a brush as one line along the path, whose width, opacity and color follow the particle's size, opacity and
  /// color at each place along it, and which fades to nothing at its far end. A texture is not used for a trail.
  trail: f32,
  /// How hard the edge of a dot is, from 0.0 (soft) to 1.0 (hard). A texture does not use it.
  hardness: f32,
  /// Whether `progress` goes round and round: a particle's age is `progress` plus its own place in the cycle (spread over the life by `age_spread`),
  /// started over at birth when it passes the end of its life, so particles go on being made.
  looping: bool,
  /// How soft the edges of a trail are, from 0.0 (a soft fade) to 1.0 (a hard edge).
  trail_hardness: f32,
  /// How much a trail fades toward its tail: 1.0 fades to nothing at the far end, 0.0 does not fade at all.
  trail_fade: f32,
  /// How a particle is blended with what is under it.
  blend: Blend,
  /// Turns each particle's hue by this many degrees over its life.
  hue_variation: CurveRange,
  /// A pull down the image, in pixels per whole life squared. Negative pulls up.
  gravity: f32,
  /// Spin around the velocity center, in degrees per whole life, and push away from it (negative toward it),
  /// in pixels per whole life.
  orbital_velocity: CurveRange,
  radial_velocity: CurveRange,
  /// What orbital and radial velocity move around; `None` is the middle of the emitter, or the point a
  /// sub-emitter's particle was born at.
  velocity_center: Option<(f32, f32)>,
  /// Smooth noise that pushes particles about: how far in pixels over a life, and the size of one swirl in pixels.
  noise: Option<(CurveRange, f32)>,
  noise_octaves: u32,
  noise_scroll: (f32, f32),
  /// Whether the strength of the noise is a share of one swirl in place of pixels.
  noise_damping: bool,
  /// A box particles bounce around in, in pixels: `(left, top, right, bottom)`.
  bounds: Option<(f32, f32, f32, f32)>,
  /// How size, rotation and color follow how fast a particle is going.
  speed_response: Option<SpeedResponse>,
  /// Noise that changes the size and rotation of particles where they are.
  pose_noise: Option<PoseNoise>,
  /// How much longer a particle is for each pixel over a whole life it goes: it is turned the way it goes and drawn longer the faster it goes.
  velocity_stretch: f32,
  /// A texture that is a grid of frames, `columns` x `rows`, played along each particle's life `frame_cycles` times.
  frames: (u32, u32),
  frame_cycles: f32,
  /// How quickly the start speed is lost over a life, and a constant push in x and y in pixels over a whole life squared.
  drag: f32,
  wind: (f32, f32),
  /// When particles are born and how long they live, in seconds. With it, `count`, `progress` and `age_spread` are not used: the particles are the
  /// things of the emission, and `time` says the moment that is drawn. A sub-emitter does not use it.
  emission: Option<Schedule>,
  time: f32,
  /// Opacity over its life (0.0 to 1.0).
  opacity: CurveRange,
  /// How far along its life the system is (0.0 to 1.0).
  progress: f32,
  /// How far each particle's own age may differ from `progress`, so they are not all the same age.
  age_spread: f32,
  /// The speed each particle is born with along x and along y, in pixels per whole life. It stays the same over the
  /// life; `velocity_over_lifetime` is added to it.
  start_speed: (CurveRange, CurveRange),
  /// A launch added to the start speed: each particle picks its own speed and direction.
  launch: Option<Impulse>,
  /// Velocity along x and along y over a particle's life, in pixels per whole life. A particle's position is its
  /// start speed plus this added up over its age, so a cosine-shaped x curve makes an "S".
  velocity_over_lifetime: (CurveRange, CurveRange),
  /// Systems born from each particle once it reaches the given age (0.0 to 1.0).
  sub_emitters: Vec<(f32, ParticleSystem)>,
  /// The area, mask and hardware the effect is limited to, as for any abra effect.
  options: Options,
}

/// The untouched pixels particles take their color from, when they are at hand, and the size of the image they are made for. The
/// GPU makes particles without the pixels: a color taken from the picture is then left for its shader to read.
struct Surface<'a> {
  source: Option<&'a Image>,
  width: u32,
  height: u32,
}

/// A particle at one age, worked out but not yet handed to the renderer.
struct Placed {
  x: f32,
  y: f32,
  size: f32,
  opacity: f32,
  color: (u8, u8, u8),
  rotation: f32,
  stretch: f32,
  frame: u32,
}

/// What a particle picked at random, from 0.0 to 1.0, and the start speed that came of it.
struct Picks {
  size: f32,
  opacity: f32,
  x: f32,
  y: f32,
  hue: f32,
  orbital: f32,
  radial: f32,
  noise: f32,
  rotation: f32,
  speed: (f32, f32),
}

pub fn particle_system() -> ParticleSystem {
  // Making the first batch of sprites has the GPU compile the sprite shader in the background, so drawing a system does not wait for it.
  sprites();
  ParticleSystem {
    emitter: Emitter::Image,
    seed: 0,
    count: 200,
    size: 5.0.into(),
    start_color: None,
    color_over_lifetime: None,
    texture: None,
    rotation: 0.0.into(),
    trail: 0.0,
    hardness: 0.0,
    looping: false,
    trail_hardness: 0.5,
    trail_fade: 1.0,
    blend: Blend::default(),
    hue_variation: 0.0.into(),
    gravity: 0.0,
    orbital_velocity: 0.0.into(),
    radial_velocity: 0.0.into(),
    velocity_center: None,
    noise: None,
    noise_octaves: 1,
    noise_scroll: (0.0, 0.0),
    noise_damping: false,
    bounds: None,
    speed_response: None,
    pose_noise: None,
    velocity_stretch: 0.0,
    frames: (1, 1),
    frame_cycles: 1.0,
    drag: 0.0,
    wind: (0.0, 0.0),
    emission: None,
    time: 0.0,
    opacity: 1.0.into(),
    progress: 0.5,
    age_spread: 0.0,
    start_speed: (0.0.into(), 0.0.into()),
    launch: None,
    velocity_over_lifetime: (0.0.into(), 0.0.into()),
    sub_emitters: Vec::new(),
    options: None,
  }
}

impl ParticleSystem {
  /// How this system moves its particles over a life.
  fn motion(&self) -> Motion {
    let mut motion = Motion::new()
      .with_velocity(self.velocity_over_lifetime.0.clone(), self.velocity_over_lifetime.1.clone())
      .with_orbital(self.orbital_velocity.clone())
      .with_radial(self.radial_velocity.clone())
      .with_gravity(self.gravity)
      .with_drag(self.drag)
      .with_wind(self.wind.0, self.wind.1)
      .with_noise_octaves(self.noise_octaves)
      .with_noise_scroll(self.noise_scroll.0, self.noise_scroll.1)
      .with_noise_damping(self.noise_damping);
    if let Some((left, top, right, bottom)) = self.bounds {
      motion = motion.with_bounds(left, top, right, bottom);
    }
    if let Some((strength, swirl)) = &self.noise {
      motion = motion.with_noise(strength.clone(), *swirl, self.seed);
    }
    motion
  }

  /// Where a particle is born: at `p_origin` for a sub-emitter, and in the emitter otherwise. `p_light` has the places found for a luminance emitter.
  fn birth_point(&self, p_surface: &Surface, p_rng: &mut Random, p_origin: Option<(f32, f32)>, p_light: &mut std::vec::IntoIter<(f32, f32)>) -> (f32, f32) {
    let (width, height) = (p_surface.width, p_surface.height);
    match (p_origin, &self.emitter) {
      (Some(origin), _) => origin,
      (None, Emitter::Image) => (p_rng.range(0.0, width as f32), p_rng.range(0.0, height as f32)),
      (None, Emitter::Circle { center, radius }) => p_rng.point_in_circle(*center, *radius),
      (None, Emitter::Ring { center, radius, inner_radius }) => p_rng.point_in_ring(*center, *inner_radius, *radius),
      (None, Emitter::Rect(left, top, w, h)) => p_rng.point_in_rect(*left, *top, w.max(f32::EPSILON), *h),
      (None, Emitter::Line(from, to)) => p_rng.point_on_line(*from, *to),
      (None, Emitter::Luminance(_)) => p_light.next().unwrap_or_else(|| (p_rng.range(0.0, width as f32), p_rng.range(0.0, height as f32))),
    }
  }

  /// What a particle picks at random, in the order the numbers are asked: its size, opacity, the velocity in x and y, hue, orbital turn, radial push,
  /// noise and rotation, then its start speed. A launch asks for two more, after the others, so the rest of the scatter is the same with or without it.
  fn draw_picks(&self, p_rng: &mut Random, p_born: (f32, f32), p_center: (f32, f32)) -> Picks {
    let mut pick = || p_rng.float();
    let (size, opacity, x, y) = (pick(), pick(), pick(), pick());
    let (hue, orbital, radial, noise) = (pick(), pick(), pick(), pick());
    let rotation = pick();
    let (mut speed_x, mut speed_y) = (self.start_speed.0.at(0.0, pick()), self.start_speed.1.at(0.0, pick()));
    if let Some(launch) = &self.launch {
      let (launch_x, launch_y) = launch.velocity(p_born, p_center, pick(), pick());
      speed_x += launch_x;
      speed_y += launch_y;
    }
    Picks { size, opacity, x, y, hue, orbital, radial, noise, rotation, speed: (speed_x, speed_y) }
  }

  pub fn with_emitter(mut self, p_emitter: Emitter) -> Self {
    self.emitter = p_emitter;
    self
  }

  pub fn with_seed(mut self, p_seed: u64) -> Self {
    self.seed = p_seed;
    self
  }

  pub fn with_count(mut self, p_count: usize) -> Self {
    self.count = p_count;
    self
  }

  pub fn with_size(mut self, p_size: impl Into<CurveRange>) -> Self {
    self.size = p_size.into();
    self
  }

  pub fn with_start_color(mut self, p_gradient: Gradient) -> Self {
    self.start_color = Some(p_gradient);
    self
  }

  pub fn with_color_over_lifetime(mut self, p_gradient: Gradient) -> Self {
    self.color_over_lifetime = Some(p_gradient);
    self
  }

  pub fn with_texture(mut self, p_texture: Image) -> Self {
    self.texture = Some(p_texture);
    self
  }

  pub fn with_rotation(mut self, p_degrees: impl Into<CurveRange>) -> Self {
    self.rotation = p_degrees.into();
    self
  }

  /// How hard the edge of a dot is, from 0.0 (soft) to 1.0 (hard).
  pub fn with_hardness(mut self, p_hardness: f32) -> Self {
    self.hardness = p_hardness.clamp(0.0, 1.0);
    self
  }

  /// Whether `progress` goes round and round, so particles go on being made (see `looping`).
  pub fn with_looping(mut self, p_looping: bool) -> Self {
    self.looping = p_looping;
    self
  }

  pub fn with_trail(mut self, p_trail: f32) -> Self {
    self.trail = p_trail.max(0.0);
    self
  }

  pub fn with_trail_fade(mut self, p_fade: f32) -> Self {
    self.trail_fade = p_fade.clamp(0.0, 1.0);
    self
  }

  pub fn with_trail_hardness(mut self, p_hardness: f32) -> Self {
    self.trail_hardness = p_hardness.clamp(0.0, 1.0);
    self
  }

  pub fn with_blend(mut self, p_blend: impl Into<Blend>) -> Self {
    self.blend = p_blend.into();
    self
  }

  pub fn with_hue_variation(mut self, p_degrees: impl Into<CurveRange>) -> Self {
    self.hue_variation = p_degrees.into();
    self
  }

  pub fn with_gravity(mut self, p_gravity: f32) -> Self {
    self.gravity = p_gravity;
    self
  }

  pub fn with_orbital_velocity(mut self, p_degrees: impl Into<CurveRange>) -> Self {
    self.orbital_velocity = p_degrees.into();
    self
  }

  pub fn with_radial_velocity(mut self, p_pixels: impl Into<CurveRange>) -> Self {
    self.radial_velocity = p_pixels.into();
    self
  }

  pub fn with_velocity_center(mut self, p_x: f32, p_y: f32) -> Self {
    self.velocity_center = Some((p_x, p_y));
    self
  }

  pub fn with_noise(mut self, p_strength: impl Into<CurveRange>, p_swirl_size: f32) -> Self {
    self.noise = Some((p_strength.into(), p_swirl_size.max(1.0)));
    self
  }

  /// How many layers of noise are added (at least 1), each at twice the detail and half the strength of the one before.
  pub fn with_noise_octaves(mut self, p_octaves: u32) -> Self {
    self.noise_octaves = p_octaves.max(1);
    self
  }

  /// Whether the strength of the noise is a share of one swirl in place of pixels, so bigger swirls move particles farther.
  pub fn with_noise_damping(mut self, p_damping: bool) -> Self {
    self.noise_damping = p_damping;
    self
  }

  /// A box that particles bounce around in, in pixels: they bounce off its sides as off walls, without losing speed.
  pub fn with_bounds(mut self, p_left: f32, p_top: f32, p_right: f32, p_bottom: f32) -> Self {
    self.bounds = Some((p_left, p_top, p_right, p_bottom));
    self
  }

  /// Makes size, rotation and color follow how fast each particle is going: see [`SpeedResponse`].
  pub fn with_speed_response(mut self, p_response: SpeedResponse) -> Self {
    self.speed_response = Some(p_response);
    self
  }

  /// Stretches each particle along the way it goes, and turns it that way: it is `1 + speed * p_scale` times as long, with the speed in pixels over a
  /// whole life. The rotation of the particle is then added to the way it goes.
  pub fn with_velocity_stretch(mut self, p_scale: f32) -> Self {
    self.velocity_stretch = p_scale.max(0.0);
    self
  }

  /// Treats the texture as a grid of `p_columns` x `p_rows` frames and plays them along each particle's life, left to right and down.
  pub fn with_texture_frames(mut self, p_columns: u32, p_rows: u32) -> Self {
    self.frames = (p_columns.max(1), p_rows.max(1));
    self
  }

  /// How many times the frames of the texture play over a particle's life (1.0 plays them once).
  pub fn with_frame_cycles(mut self, p_cycles: f32) -> Self {
    self.frame_cycles = p_cycles.max(0.0);
    self
  }

  /// Noise that changes the size and the rotation of particles where they are: see [`PoseNoise`].
  pub fn with_pose_noise(mut self, p_noise: PoseNoise) -> Self {
    self.pose_noise = Some(p_noise);
    self
  }

  /// How far the noise slides across the image over a whole life, in swirls in x and in y.
  pub fn with_noise_scroll(mut self, p_x: f32, p_y: f32) -> Self {
    self.noise_scroll = (p_x, p_y);
    self
  }

  /// How quickly the start speed is lost over a life: 0.0 keeps it, and the higher it is the sooner a particle slows to a stop.
  pub fn with_drag(mut self, p_drag: f32) -> Self {
    self.drag = p_drag.max(0.0);
    self
  }

  /// A constant push in x and in y, in pixels over a whole life squared, besides gravity.
  pub fn with_wind(mut self, p_x: f32, p_y: f32) -> Self {
    self.wind = (p_x, p_y);
    self
  }

  /// Makes particles over time: they are the things of `p_emission` (its bursts and its rate), each alive for a lifetime it picks, and only the ones
  /// alive at the `with_time` moment are drawn. Replaces `count`, `progress` and `age_spread`. A particle's random numbers are its own, so what a
  /// particle looks like does not change with the time.
  pub fn with_emission(mut self, p_emission: Schedule) -> Self {
    self.emission = Some(p_emission);
    self
  }

  /// The moment of an emission that is drawn, in seconds.
  pub fn with_time(mut self, p_seconds: f32) -> Self {
    self.time = p_seconds;
    self
  }

  pub fn with_opacity(mut self, p_opacity: impl Into<CurveRange>) -> Self {
    self.opacity = p_opacity.into();
    self
  }

  pub fn with_progress(mut self, p_progress: f32) -> Self {
    self.progress = p_progress;
    self
  }

  pub fn with_age_spread(mut self, p_spread: f32) -> Self {
    self.age_spread = p_spread.max(0.0);
    self
  }

  pub fn with_start_speed(mut self, p_x: impl Into<CurveRange>, p_y: impl Into<CurveRange>) -> Self {
    self.start_speed = (p_x.into(), p_y.into());
    self
  }

  /// Throws each particle when it is born, on top of its start speed: see [`Impulse`].
  pub fn with_launch(mut self, p_launch: Impulse) -> Self {
    self.launch = Some(p_launch);
    self
  }

  pub fn with_velocity_over_lifetime(mut self, p_x: impl Into<CurveRange>, p_y: impl Into<CurveRange>) -> Self {
    self.velocity_over_lifetime = (p_x.into(), p_y.into());
    self
  }

  /// Adds a child system to every particle, born where the particle is once it reaches `p_age`
  /// (0.0 to 1.0); particles younger than that spawn nothing. Each call adds another one. The child's `count`
  /// is per parent particle and its emitter is not used. Its other values, `progress` included, are its own.
  pub fn with_sub_emitter(mut self, p_age: f32, p_system: ParticleSystem) -> Self {
    self.sub_emitters.push((p_age.clamp(0.0, 1.0), p_system));
    self
  }

  /// Makes `count` particles born in the emitter, or at `p_origin` when this is a sub-emitter, and adds them to
  /// `p_particles` as sprites in the order they are drawn, a particle's children before it. A trail goes to `p_trails`.
  fn collect(
    &self,
    p_surface: &Surface,
    p_rng: &mut Random,
    p_origin: Option<(f32, f32)>,
    p_particles: &mut Vec<Sprite>,
    p_trails: &mut Vec<SpriteTrail>,
  ) {
    let (width, height) = (p_surface.width, p_surface.height);
    // Where a luminance emitter's particles are born, found once for all of them from the pixels of the picture.
    let mut light = match (&self.emitter, p_origin, p_surface.source) {
      (Emitter::Luminance(min), None, Some(source)) => p_rng.points_in_light(source, *min, self.count).into_iter(),
      _ => Vec::new().into_iter(),
    };
    // An emission makes the things it says, and gives each its own random numbers, so a particle is the same whatever the time.
    let emission = self.emission.as_ref().filter(|_| p_origin.is_none());
    for index in 0..emission.map_or(self.count, Schedule::count) {
      let mut own = Random::new(self.seed.wrapping_add((index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
      let p_rng = if emission.is_some() { &mut own } else { &mut *p_rng };
      let (bx, by) = self.birth_point(p_surface, p_rng, p_origin, &mut light);
      let center = self.velocity_center.or(p_origin).unwrap_or_else(|| self.emitter.center(width, height));

      let spread = if self.age_spread > 0.0 { p_rng.range(-self.age_spread, self.age_spread) } else { 0.0 };
      let phase = if self.age_spread > 0.0 { (spread + self.age_spread) / (2.0 * self.age_spread) } else { 0.0 };
      let mut age = if self.looping { (self.progress + phase) % 1.0 } else { (self.progress + spread).clamp(0.0, 1.0) };
      let picks = self.draw_picks(p_rng, (bx, by), center);
      let Picks { size: size_pick, opacity: opacity_pick, x: x_pick, y: y_pick, hue: hue_pick, orbital: orbital_pick, radial: radial_pick, noise: noise_pick, rotation: rotation_pick, speed: (speed_x, speed_y) } = picks;
      let alive = emission.map(|emission| emission.age(index, self.time, p_rng.float()));
      let motion = self.motion();
      let mover = Mover {
        born: (bx, by),
        center,
        speed: (speed_x, speed_y),
        picks: MoverPicks { velocity_x: x_pick, velocity_y: y_pick, orbital: orbital_pick, radial: radial_pick, noise: noise_pick },
      };
      let at_age = |p_age: f32| motion.at(&mover, p_age);
      match alive {
        Some(Some(lived)) => age = lived,
        // Not alive at this moment: its numbers are all made, so what is left is to draw nothing.
        Some(None) => continue,
        None => {}
      }

      // Children go down first so the parent is drawn over them.
      for (trigger, child) in &self.sub_emitters {
        if age >= *trigger {
          child.collect(p_surface, p_rng, Some(at_age(*trigger)), p_particles, p_trails);
        }
      }

      let start = self.start_color.as_ref().map(|gradient| gradient.color_at(p_rng.float()));
      let from_picture = self.start_color.is_none() && self.color_over_lifetime.is_none() && self.texture.is_none();
      let birth = (from_picture && p_surface.source.is_none()).then_some((bx, by));

      // The particle as it was at `p_age`, with its opacity scaled by `p_fade`.
      let at = |p_age: f32, p_fade: f32| {
        let lifetime = self.color_over_lifetime.as_ref().map(|gradient| gradient.color_at(p_age));
        let (r, g, b, a) = match (start, lifetime) {
          (Some(start), Some(lifetime)) => {
            let (start, lifetime) = (start.rgba(), lifetime.rgba());
            let product = |a: u8, b: u8| (a as f32 * b as f32 / 255.0) as u8;
            (product(start.0, lifetime.0), product(start.1, lifetime.1), product(start.2, lifetime.2), product(start.3, lifetime.3))
          }
          (Some(color), None) | (None, Some(color)) => color.rgba(),
          (None, None) if self.texture.is_some() => (255, 255, 255, 255),
          // The picture's own color at the birth point, read now when the picture is at hand and left to the shader when it is not.
          (None, None) => match p_surface.source {
            Some(source) => {
              let at = (((by as i64).clamp(0, height as i64 - 1) as usize) * width as usize
                + (bx as i64).clamp(0, width as i64 - 1) as usize)
                * 4;
              let source = source.rgba();
              (source[at], source[at + 1], source[at + 2], 255)
            }
            None => (0, 0, 0, 255),
          },
        };
        let (r, g, b, _) = Color::from_rgba(r, g, b, a).with_hue_rotated(self.hue_variation.at(p_age, hue_pick)).rgba();
        let (x, y) = at_age(p_age);
        let mut size = self.size.at(p_age, size_pick).max(0.0);
        let mut opacity = self.opacity.at(p_age, opacity_pick).clamp(0.0, 1.0) * a as f32 / 255.0 * p_fade;
        let mut rotation = self.rotation.at(p_age, rotation_pick);
        let mut color = [r as f32, g as f32, b as f32];
        if let Some(response) = &self.speed_response {
          (size, rotation, color, opacity) = response.apply(motion.speed_at(&mover, p_age), size, rotation, color, opacity);
        }
        if let Some(noise) = &self.pose_noise {
          (size, rotation) = noise.apply((x, y), size, rotation);
        }
        let mut stretch = 0.0;
        if self.velocity_stretch > 0.0 {
          let (vx, vy) = motion.velocity_at(&mover, p_age);
          stretch = vx.hypot(vy) * self.velocity_stretch;
          if stretch > 0.0 {
            rotation += vy.atan2(vx).to_degrees();
          }
        }
        let frames = self.frames.0 * self.frames.1;
        let frame = if frames > 1 { ((p_age * self.frame_cycles).fract() * frames as f32) as u32 } else { 0 };
        Placed { x, y, size, opacity, color: (color[0] as u8, color[1] as u8, color[2] as u8), rotation, stretch, frame }
      };

      // The streak is the particle at earlier ages, as a line from its tail to its head. Each sample says how wide, how opaque and
      // what color the line is there. A line too short to be seen is left out.
      let head = at(age, 1.0);
      let span = self.trail.min(age);
      if span > 0.0 {
        const SAMPLES: usize = 24;
        let samples: Vec<Placed> = (0..=SAMPLES)
          .map(|i| {
            let behind = 1.0 - i as f32 / SAMPLES as f32;
            at(age - span * behind, 1.0 - self.trail_fade * behind)
          })
          .collect();
        let length: f32 = samples.windows(2).map(|pair| ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt()).sum();
        if length >= 0.5 {
          let points = samples.iter().map(|sample| TrailPoint::new(sample.x, sample.y, sample.size * 2.0).with_opacity(sample.opacity).with_color(sample.color));
          p_trails.push(SpriteTrail::new(points.collect()).with_hardness(self.trail_hardness).with_blend(self.blend));
        }
      }
      let mut sprite = Sprite::new(head.x, head.y, head.size).with_opacity(head.opacity).with_color(head.color).with_rotation(head.rotation).with_stretch(head.stretch).with_hardness(self.hardness).with_blend(self.blend);
      if let Some(texture) = &self.texture {
        sprite = sprite.with_texture(texture.clone()).with_frame(self.frames.0, self.frames.1, head.frame);
      }
      if let Some((x, y)) = birth {
        sprite = sprite.with_color_from_image_at(x, y);
      }
      p_particles.push(sprite);
    }
  }
}

/// A particle system is an abra effect: `system.apply(&mut image)` draws it on the image now, on the GPU when the system can run there
/// and a GPU is ready, and on the CPU otherwise, and `system.applied(&image)` does the same to a copy. The same seed gives the same
/// result. Trails are drawn first, under all the particles.
impl Effect for ParticleSystem {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// Particles are placed in pixels of the whole image, so an area crops what is drawn and never moves it.
  fn positional(&self) -> bool {
    true
  }

  /// The GPU draws the system when it can: see [`ParticleSystem::gpu_supported`].
  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    self.gpu_supported().then_some(self)
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<u32>();
    let source = p_image.clone();
    let (mut particles, mut trails) = (Vec::new(), Vec::new());
    let mut rng = Random::new(self.seed);
    self.collect(&Surface { source: Some(&source), width, height }, &mut rng, None, &mut particles, &mut trails);
    sprites().with_sprites(particles).with_trails(trails).cpu_processor(p_image);
  }
}

impl ParticleSystem {
  /// Whether the GPU can draw the system: what the shader cannot do is a luminance emitter, which picks pixels of the picture that
  /// the particles are made without, a color taken from the picture that is turned in hue or put in a trail, a blend mode that has
  /// no shader, and more particles than a texture holds. Those are drawn by the CPU.
  fn gpu_supported(&self) -> bool {
    self.gpu_supported_as(false) && self.gpu_words() <= SPRITE_MAX_WORDS
  }

  fn gpu_supported_as(&self, p_child: bool) -> bool {
    let from_picture = self.start_color.is_none() && self.color_over_lifetime.is_none() && self.texture.is_none();
    let emitter = p_child || !matches!(self.emitter, Emitter::Luminance(_));
    let colors = !from_picture || (self.hue_variation.is_zero() && self.trail <= 0.0 && !self.speed_response.as_ref().is_some_and(SpeedResponse::changes_color));
    emitter && colors && self.blend.shader_code().is_some() && self.sub_emitters.iter().all(|(_, child)| child.gpu_supported_as(true))
  }

  /// The most words the system's particles, trails and textures take, worked out from the settings without making the particles.
  fn gpu_words(&self) -> usize {
    fn per_particle(p_system: &ParticleSystem) -> usize {
      // A trail is a line through 25 points.
      let trail = if p_system.trail > 0.0 { TRAIL_WORDS + 25 * TRAIL_POINT_WORDS } else { 0 };
      let children: usize = p_system.sub_emitters.iter().map(|(_, child)| child.count.saturating_mul(per_particle(child))).sum();
      SPRITE_WORDS + trail + children
    }
    fn textures(p_system: &ParticleSystem, p_seen: &mut Vec<(u64, usize)>) {
      if let Some(texture) = &p_system.texture
        && !p_seen.iter().any(|(version, _)| *version == texture.version())
      {
        let (width, height) = texture.dimensions::<usize>();
        p_seen.push((texture.version(), width.saturating_mul(height)));
      }
      for (_, child) in &p_system.sub_emitters {
        textures(child, p_seen);
      }
    }
    let mut seen = Vec::new();
    textures(self, &mut seen);
    self.emission.as_ref().map_or(self.count, Schedule::count).saturating_mul(per_particle(self)).saturating_add(seen.iter().map(|(_, words)| words).sum::<usize>())
  }
}

/// The particles are made on the CPU, which has to, since they share a random stream, and the sprite renderer's shader draws them.
impl GpuProcessor for ParticleSystem {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let (mut particles, mut trails) = (Vec::new(), Vec::new());
    let mut rng = Random::new(self.seed);
    self.collect(&Surface { source: None, width: p_width, height: p_height }, &mut rng, None, &mut particles, &mut trails);
    sprites().with_sprites(particles).with_trails(trails).passes(p_width, p_height)
  }
}
