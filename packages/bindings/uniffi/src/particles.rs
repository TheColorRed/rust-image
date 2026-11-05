//! The particle system (`particle-system`): `Swarm`, many particles made once and drawn at any moment of their life in one call. A list of the wrong
//! length panics. `usize` is a `u32`.
//!
//! What is not exported, and why:
//! - `SwarmStyle` is a list of numbers in `Swarm::new`'s `look`, as `Frame` of the `animation` crate is not exported: `Swarm::sprites` turns it into
//!   the sprites the drawing library draws, so thousands of particles never cross as numbers.
//! - A `ParticleSystem` is described here and made into a `Swarm` by `ParticleSystem::swarm`, so an app does not plan the particles itself.

use std::sync::Arc;

use abra_particle_system as particles;

use crate::sprites::{Blend, CurveRange, Sprites};
use crate::{Gradient, Image, Impulse, Motion, PoseNoise, Schedule, SpeedResponse};

/// Many particles that live and die together, made once and drawn at any moment of their life in one call. Made by [`Swarm::new`] from the numbers of
/// every particle (22 for each: see the library's `PARTICLE_VALUES`), the way they move, and the ranges that shape them over a life.
#[derive(Clone, Debug, uniffi::Object)]
pub struct Swarm(particles::Swarm);

#[uniffi::export]
impl Swarm {
  /// `look` is 7 numbers: whether the progress goes round and round (`1` or `0`), the trail (as a share of a life, `0` for none), how much a trail fades,
  /// its hardness, the hardness of a dot, whether a color is read from the picture where it was born (`1` or `0`), and the group, a whole number that
  /// says which of the `textures` of [`Swarm::sprites`] these particles are drawn as. A list of the wrong length panics, and so do particles that are not
  /// a whole number of them.
  #[uniffi::constructor]
  pub fn new(
    motion: Arc<Motion>, particles: Vec<f32>, look: Vec<f32>, size: Arc<CurveRange>, opacity: Arc<CurveRange>, rotation: Arc<CurveRange>, hue: Arc<CurveRange>,
  ) -> Self {
    let [looping, trail, trail_fade, trail_hardness, hardness, picture_color, group] = look[..] else {
      panic!("a look is 7 numbers");
    };
    let style = particles::SwarmStyle { hardness, trail_hardness, picture_color: picture_color != 0.0, ..particles::SwarmStyle::default() };
    Self(particles::Swarm::new(
      motion.core(),
      &particles,
      looping != 0.0,
      trail,
      trail_fade,
      group as u32,
      style,
      size.core(),
      opacity.core(),
      rotation.core(),
      hue.core(),
    ))
  }

  /// The color the particles are multiplied by over a life: 257 colors of four numbers each (`red`, `green`, `blue`, `alpha`, 0 to 255) from birth to
  /// the end. Any other length panics.
  pub fn with_lifetime_colors(&self, colors: Vec<f32>) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_lifetime_colors(&colors)))
  }

  /// Makes the size, rotation and color of the particles follow how fast they go.
  pub fn with_speed_response(&self, response: Arc<SpeedResponse>) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_speed_response(response.core())))
  }

  /// Noise that changes the size and the rotation of the particles where they are.
  pub fn with_pose_noise(&self, noise: Arc<PoseNoise>) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_pose_noise(noise.core())))
  }

  /// Stretches each particle along the way it goes, and turns it that way: it is `1 + speed * scale` times as long, with the speed in pixels over a whole
  /// life.
  pub fn with_velocity_stretch(&self, scale: f32) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_velocity_stretch(scale)))
  }

  /// Treats the texture of this swarm as a grid of `columns` x `rows` frames, played along each particle's life `cycles` times.
  pub fn with_frames(&self, columns: u32, rows: u32, cycles: f32) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_frames(columns, rows, cycles)))
  }

  /// Makes the particles last over time: they are the things of `schedule` (it must make as many as there are particles), each alive for a lifetime
  /// picked with its entry of `lifetime_picks` (0 to 1). From then on `progress` of [`Swarm::sprites`] is the time in seconds. Lists that do not fit panic.
  pub fn with_schedule(&self, schedule: Arc<Schedule>, lifetime_picks: Vec<f32>) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_schedule(schedule.core(), &lifetime_picks)))
  }

  /// A swarm born from the particles of this one once each is `trigger` (0 to 1) of the way through its life. `counts` says how many particles of `child`
  /// belong to each particle of this swarm. Counts that do not fit panic.
  pub fn with_child(&self, trigger: f32, child: Arc<Swarm>, counts: Vec<u32>) -> Arc<Swarm> {
    Arc::new(Swarm(self.0.clone().with_child(trigger, child.0.clone(), &counts)))
  }

  /// How many particles there are, not counting the ones born from them.
  pub fn len(&self) -> u32 {
    self.0.len() as u32
  }

  /// The sprites and trails of the swarm `progress` (0 to 1) of the way through its life, or at that many seconds when it has an schedule, to draw
  /// with `blend`. `textures` are the pictures the groups
  /// are drawn as, by group number (a missing one or `None` is a soft dot).
  pub fn sprites(&self, progress: f32, blend: Arc<Blend>, textures: Vec<Option<Arc<Image>>>) -> Arc<Sprites> {
    let textures: Vec<Option<_>> = textures.iter().map(|texture| texture.as_ref().map(|texture| texture.core())).collect();
    Sprites::from_core(self.0.sprites(progress, blend.core(), &textures))
  }
}

/// Where particles are born, in pixels of the image. A point is two numbers, `[x, y]`; a list of any other length panics.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum Emitter {
  /// Anywhere on the image.
  Image,
  /// Inside a circle.
  Circle { center: Vec<f32>, radius: f32 },
  /// Inside a rectangle.
  Rect { left: f32, top: f32, width: f32, height: f32 },
  /// On a ring between `inner_radius` and `radius`.
  Ring { center: Vec<f32>, radius: f32, inner_radius: f32 },
  /// Along a line from one point to another.
  Line { from: Vec<f32>, to: Vec<f32> },
  /// Anywhere on the image where the pixel is at least this light (0 to 1).
  Luminance { min: f32 },
}

fn point(values: &[f32]) -> (f32, f32) {
  let [x, y] = values[..] else {
    panic!("a point is 2 numbers");
  };
  (x, y)
}

impl Emitter {
  fn core(&self) -> particles::Emitter {
    match self {
      Emitter::Image => particles::Emitter::Image,
      Emitter::Circle { center, radius } => particles::Emitter::Circle { center: point(center), radius: *radius },
      Emitter::Rect { left, top, width, height } => particles::Emitter::Rect(*left, *top, *width, *height),
      Emitter::Ring { center, radius, inner_radius } => particles::Emitter::Ring { center: point(center), radius: *radius, inner_radius: *inner_radius },
      Emitter::Line { from, to } => particles::Emitter::Line(point(from), point(to)),
      Emitter::Luminance { min } => particles::Emitter::Luminance(*min),
    }
  }
}

/// A particle system: where particles are born and how each one lives. It is described here and then made into a [`Swarm`] with [`ParticleSystem::swarm`],
/// which works out where every particle is born and what it picks at random, once, and draws them at any moment of their life.
#[derive(Clone, Debug, uniffi::Object)]
pub struct ParticleSystem(particles::ParticleSystem);

#[uniffi::export]
impl ParticleSystem {
  /// A system of 200 soft dots over the whole image.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(particles::particle_system())
  }

  /// Where particles are born.
  pub fn with_emitter(&self, emitter: Emitter) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_emitter(emitter.core())))
  }

  pub fn with_seed(&self, seed: u64) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_seed(seed)))
  }

  pub fn with_count(&self, count: u32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_count(count as usize)))
  }

  /// The size of a particle in pixels over its life (the radius of a dot).
  pub fn with_size(&self, size: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_size(size.core())))
  }

  /// A gradient to pick each particle's color from.
  pub fn with_start_color(&self, gradient: Arc<Gradient>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_start_color(gradient.0.clone())))
  }

  /// The gradient a particle's color follows over its life. It multiplies the start color, and its alpha multiplies the opacity.
  pub fn with_color_over_lifetime(&self, gradient: Arc<Gradient>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_color_over_lifetime(gradient.0.clone())))
  }

  /// The picture each particle is drawn as, instead of a dot.
  pub fn with_texture(&self, texture: Arc<Image>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_texture(texture.core())))
  }

  /// Degrees over a particle's life.
  pub fn with_rotation(&self, degrees: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_rotation(degrees.core())))
  }

  /// How far back along its path a particle leaves a streak, as a share of its life (0 for none).
  pub fn with_trail(&self, trail: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_trail(trail)))
  }

  /// How much a trail fades toward its tail: 1 fades to nothing, 0 does not fade.
  pub fn with_trail_fade(&self, fade: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_trail_fade(fade)))
  }

  /// How soft the edges of a trail are, from 0 (a soft fade) to 1 (a hard edge).
  pub fn with_trail_hardness(&self, hardness: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_trail_hardness(hardness)))
  }

  /// How hard the edge of a dot is, from 0 (soft) to 1 (hard).
  pub fn with_hardness(&self, hardness: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_hardness(hardness)))
  }

  /// Whether `progress` goes round and round, so particles go on being made.
  pub fn with_looping(&self, looping: bool) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_looping(looping)))
  }

  /// How particles are blended with what is under them.
  pub fn with_blend(&self, blend: Arc<Blend>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_blend(blend.core())))
  }

  /// Degrees the hue turns over a particle's life.
  pub fn with_hue_variation(&self, degrees: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_hue_variation(degrees.core())))
  }

  /// A pull down the image, in pixels over a whole life squared. Negative pulls up.
  pub fn with_gravity(&self, gravity: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_gravity(gravity)))
  }

  /// How quickly the start speed is lost over a life.
  pub fn with_drag(&self, drag: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_drag(drag)))
  }

  /// A constant push in x and in y, in pixels over a whole life squared.
  pub fn with_wind(&self, x: f32, y: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_wind(x, y)))
  }

  /// Degrees spun around the velocity center over a life.
  pub fn with_orbital_velocity(&self, degrees: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_orbital_velocity(degrees.core())))
  }

  /// Pixels pushed away from the velocity center over a life (negative toward it).
  pub fn with_radial_velocity(&self, pixels: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_radial_velocity(pixels.core())))
  }

  /// What orbital and radial velocity move around, in place of the middle of the emitter.
  pub fn with_velocity_center(&self, x: f32, y: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_velocity_center(x, y)))
  }

  /// Smooth noise that pushes particles about: how far over a life and the size of one swirl in pixels.
  pub fn with_noise(&self, strength: Arc<CurveRange>, swirl_size: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_noise(strength.core(), swirl_size)))
  }

  pub fn with_noise_octaves(&self, octaves: u32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_noise_octaves(octaves)))
  }

  pub fn with_noise_scroll(&self, x: f32, y: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_noise_scroll(x, y)))
  }

  pub fn with_noise_damping(&self, damping: bool) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_noise_damping(damping)))
  }

  /// A box particles bounce around in, in pixels.
  pub fn with_bounds(&self, left: f32, top: f32, right: f32, bottom: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_bounds(left, top, right, bottom)))
  }

  /// Size, rotation and color that follow how fast a particle goes.
  pub fn with_speed_response(&self, response: Arc<SpeedResponse>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_speed_response(response.core())))
  }

  /// Noise that changes the size and the rotation of particles where they are.
  pub fn with_pose_noise(&self, noise: Arc<PoseNoise>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_pose_noise(noise.core())))
  }

  /// Longer along the way a particle goes, and turned that way: `1 + speed * scale` times as long.
  pub fn with_velocity_stretch(&self, scale: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_velocity_stretch(scale)))
  }

  /// Treats the texture as a grid of `columns` x `rows` frames and plays them along each particle's life.
  pub fn with_texture_frames(&self, columns: u32, rows: u32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_texture_frames(columns, rows)))
  }

  /// How many times the frames of the texture play over a particle's life.
  pub fn with_frame_cycles(&self, cycles: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_frame_cycles(cycles)))
  }

  /// Particles born over time: they are the things of `schedule`. From then on a swarm of the system is drawn at a time in seconds.
  pub fn with_emission(&self, schedule: Arc<Schedule>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_emission(schedule.core())))
  }

  /// Opacity over a particle's life, 0 to 1.
  pub fn with_opacity(&self, opacity: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_opacity(opacity.core())))
  }

  /// How far along its life the system is, 0 to 1, when it is drawn directly. A swarm is drawn at any moment of the life, so this does not matter to it.
  pub fn with_progress(&self, progress: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_progress(progress)))
  }

  /// How far each particle's own age may differ from the progress, so they are not all one age.
  pub fn with_age_spread(&self, spread: f32) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_age_spread(spread)))
  }

  /// The speed each particle is born with along x and along y, in pixels over a whole life.
  pub fn with_start_speed(&self, x: Arc<CurveRange>, y: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_start_speed(x.core(), y.core())))
  }

  /// Throws each particle at its own speed and direction.
  pub fn with_launch(&self, impulse: Arc<Impulse>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_launch(impulse.core())))
  }

  /// Velocity along x and along y over a particle's life, in pixels over a whole life.
  pub fn with_velocity_over_lifetime(&self, x: Arc<CurveRange>, y: Arc<CurveRange>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_velocity_over_lifetime(x.core(), y.core())))
  }

  /// Another system born from each particle once it reaches `age` (0 to 1). The child's count is for each parent particle and its emitter is not used.
  pub fn with_sub_emitter(&self, age: f32, system: Arc<ParticleSystem>) -> Arc<ParticleSystem> {
    Arc::new(ParticleSystem(self.0.clone().with_sub_emitter(age, system.0.clone())))
  }

  /// A swarm of the system's particles for a photo, worked out once and then drawn at any moment of the life with [`Swarm::sprites`]. The photo is
  /// the picture the particles are for: its size, and its pixels for an emitter that is its bright parts. Use [`ParticleSystem::textures`] for the
  /// pictures to draw with.
  pub fn swarm(&self, photo: Arc<Image>) -> Arc<Swarm> {
    let photo = photo.core();
    let (width, height) = photo.dimensions::<u32>();
    Arc::new(Swarm(self.0.swarm(width, height, Some(&photo)).0))
  }

  /// The pictures the groups of the swarm are drawn as, by group number (a group without one is `None`, a soft dot).
  pub fn textures(&self) -> Vec<Option<Arc<Image>>> {
    self.0.textures().into_iter().map(|texture| texture.map(|texture| Arc::new(Image::from_core(texture)))).collect()
  }
}
