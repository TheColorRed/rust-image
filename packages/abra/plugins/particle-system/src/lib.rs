use abra::abra_core::image::gpu::{GpuAux, GpuPass, GpuProcessor};
use abra::abra_core::{BLEND_MODES_WGSL, ColorStop, Fill, OpacityStop, StrokeOpacity, StrokeWidth, WidthStop};
use abra::drawing::prelude::{Blend, Brush, Compositor, Painter};
use abra::options::prelude::{Effect, Options};
use abra::prelude::*;

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

pub mod prelude {
  pub use super::{Emitter, ParticleSystem, Range, particle_system};
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

/// A value that follows a curve over a particle's life, like Unity's "random between two curves". Each particle
/// picks one random place between the `min` and `max` curves and keeps it, so no two are alike but each one
/// still changes over its life. A curve's `x` is the age from `0` (birth) to `255` (end of life) and its `y` is
/// the value. A number is a flat curve, `(0.0, 1.0)` is a flat range, a [`Curve`] is a range with no spread, and
/// `(Curve, Curve)` is the full thing, so any of them work wherever a range is asked for.
#[derive(Clone, Debug)]
pub struct Range {
  min: Curve,
  max: Curve,
  /// The area under each curve from birth up to each of `AREA_STEPS + 1` evenly spaced ages, worked out once here so
  /// that a velocity can be added up over a particle's life by looking it up instead of walking the curve.
  min_area: Vec<f32>,
  max_area: Vec<f32>,
  /// Both curves are zero all the way along.
  zero: bool,
}

/// How many pieces a life is cut into when a range's area is worked out.
const AREA_STEPS: usize = 256;

impl Range {
  fn new(p_min: Curve, p_max: Curve) -> Self {
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
    Range { min_area: area(&p_min), max_area: area(&p_max), zero: zero(&p_min) && zero(&p_max), min: p_min, max: p_max }
  }

  /// Whether the value is zero at every age, for every particle.
  fn is_zero(&self) -> bool {
    self.zero
  }

  /// The value at `p_age` (0.0 to 1.0) for the particle that picked `p_pick` (0.0 for `min`, 1.0 for `max`).
  fn at(&self, p_age: f32, p_pick: f32) -> f32 {
    let (low, high) = (self.min.value(p_age * 255.0), self.max.value(p_age * 255.0));
    low + (high - low) * p_pick
  }

  /// The value added up from birth to `p_age`, for a range that is a velocity. The area under a curve between two
  /// others is the same mix of their areas, so the picked particle's area is read from the two tables.
  fn area(&self, p_age: f32, p_pick: f32) -> f32 {
    let position = p_age.clamp(0.0, 1.0) * AREA_STEPS as f32;
    let (step, fraction) = (position as usize, position.fract());
    let next = (step + 1).min(AREA_STEPS);
    let read = |table: &[f32]| table[step] + (table[next] - table[step]) * fraction;
    let (low, high) = (read(&self.min_area), read(&self.max_area));
    low + (high - low) * p_pick
  }
}

impl From<f32> for Range {
  fn from(p_value: f32) -> Self {
    Range::from((p_value, p_value))
  }
}

impl From<(f32, f32)> for Range {
  fn from((p_min, p_max): (f32, f32)) -> Self {
    Range::new(Curve::new([(0.0, p_min)]), Curve::new([(0.0, p_max)]))
  }
}

impl From<Curve> for Range {
  fn from(p_curve: Curve) -> Self {
    Range::new(p_curve.clone(), p_curve)
  }
}

impl From<(Curve, Curve)> for Range {
  fn from((p_min, p_max): (Curve, Curve)) -> Self {
    Range::new(p_min, p_max)
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
  size: Range,
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
  rotation: Range,
  /// How far back along its path a particle leaves a streak, as a share of its life (0.0 for none). The streak is drawn
  /// with a brush as one line along the path, whose width, opacity and color follow the particle's size, opacity and
  /// color at each place along it, and which fades to nothing at its far end. A texture is not used for a trail.
  trail: f32,
  /// How soft the edges of a trail are, from 0.0 (a soft fade) to 1.0 (a hard edge).
  trail_hardness: f32,
  /// How much a trail fades toward its tail: 1.0 fades to nothing at the far end, 0.0 does not fade at all.
  trail_fade: f32,
  /// How a particle is blended with what is under it.
  blend: Blend,
  /// Turns each particle's hue by this many degrees over its life.
  hue_variation: Range,
  /// A pull down the image, in pixels per whole life squared. Negative pulls up.
  gravity: f32,
  /// Spin around the velocity center, in degrees per whole life, and push away from it (negative toward it),
  /// in pixels per whole life.
  orbital_velocity: Range,
  radial_velocity: Range,
  /// What orbital and radial velocity move around; `None` is the middle of the emitter, or the point a
  /// sub-emitter's particle was born at.
  velocity_center: Option<(f32, f32)>,
  /// Smooth noise that pushes particles about: how far in pixels over a life, and the size of one swirl in pixels.
  noise: Option<(Range, f32)>,
  /// Opacity over its life (0.0 to 1.0).
  opacity: Range,
  /// How far along its life the system is (0.0 to 1.0).
  progress: f32,
  /// How far each particle's own age may differ from `progress`, so they are not all the same age.
  age_spread: f32,
  /// The speed each particle is born with along x and along y, in pixels per whole life. It stays the same over the
  /// life; `velocity_over_lifetime` is added to it.
  start_speed: (Range, Range),
  /// Velocity along x and along y over a particle's life, in pixels per whole life. A particle's position is its
  /// start speed plus this added up over its age, so a cosine-shaped x curve makes an "S".
  velocity_over_lifetime: (Range, Range),
  /// Systems born from each particle once it reaches the given age (0.0 to 1.0).
  sub_emitters: Vec<(f32, ParticleSystem)>,
  /// The area, mask and hardware the effect is limited to, as for any abra effect.
  options: Options,
}

/// The untouched pixels particles take their color from, when they are at hand, and the size of the image they are made for. The
/// GPU makes particles without the pixels: a color taken from the picture is then left for its shader to read.
struct Surface<'a> {
  source: Option<&'a [u8]>,
  width: u32,
  height: u32,
}

/// One place along a trail: where it is, how wide the line is there, how opaque and what color.
struct TrailSample {
  x: f32,
  y: f32,
  width: f32,
  opacity: f32,
  color: (u8, u8, u8),
}

/// A trail, made but not yet drawn: the line a particle took from its tail to its head, at a number of places.
struct Trail<'a> {
  /// The system it came from, which says how it is drawn: its hardness and its blend.
  system: &'a ParticleSystem,
  samples: Vec<TrailSample>,
}

impl Trail<'_> {
  /// How far along the line each sample is, by length, from `0.0` at the tail to `1.0` at the head.
  fn times(&self) -> Vec<f32> {
    let mut along = vec![0.0f32];
    for pair in self.samples.windows(2) {
      along.push(along.last().unwrap() + ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt());
    }
    let length = *along.last().unwrap();
    along.iter().map(|distance| distance / length).collect()
  }

  /// The line as a path, and the brush that draws it, for the CPU.
  fn stroke(&self) -> (Path, Brush<'static>) {
    let times = self.times();
    let samples = &self.samples;
    // The line curves through the middle of each step, so a path of straight pieces still comes out smooth.
    let mut path = Path::new();
    path.move_to((samples[0].x, samples[0].y));
    for pair in samples.windows(2) {
      path.quad_to((pair[0].x, pair[0].y), ((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0));
    }
    let head = samples.last().unwrap();
    path.line_to((head.x, head.y));
    let gradient = Gradient::new(
      samples.iter().zip(&times).map(|(sample, time)| ColorStop::new(Color::from_rgb(sample.color.0, sample.color.1, sample.color.2), *time)).collect(),
    )
    // A gradient with no direction runs along a line across the top of the stroke, which is no line at all for a straight
    // vertical trail, so it is told to run along the trail.
    .with_direction(path.clone());
    let brush = Brush::new()
      .with_width(StrokeWidth::new(samples.iter().zip(&times).map(|(sample, time)| WidthStop::new(sample.width, *time)).collect()))
      .with_opacity(StrokeOpacity::new(samples.iter().zip(&times).map(|(sample, time)| OpacityStop::new(sample.opacity, *time)).collect()))
      .with_color(Fill::Gradient(Cow::Owned(gradient)))
      .with_hardness(self.system.trail_hardness)
      .with_blend(self.system.blend);
    (path, brush)
  }
}

/// One particle, with everything worked out and ready to draw. Particles are made one after another, because they share
/// a random stream, and then drawn in parallel.
struct Particle<'a> {
  /// The system it came from, which says how it is drawn: its texture and its blend.
  system: &'a ParticleSystem,
  x: f32,
  y: f32,
  size: f32,
  opacity: f32,
  color: (u8, u8, u8),
  /// The sine and cosine of its rotation.
  sin: f32,
  cos: f32,
  /// How many pixels out from `x` and `y` it can reach.
  reach: i64,
  /// Where it was born, when its color is the picture's there and was not read because the picture was not at hand.
  birth: Option<(f32, f32)>,
}

pub fn particle_system() -> ParticleSystem {
  // The first system anyone makes has the GPU compile the particle shader in the background, so drawing it does not wait for that.
  static WARMED: std::sync::Once = std::sync::Once::new();
  WARMED.call_once(|| abra::abra_core::image::gpu::register_warmup(|| particle_system().with_count(1).passes(2, 2)));
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
    trail_hardness: 0.5,
    trail_fade: 1.0,
    blend: Blend::default(),
    hue_variation: 0.0.into(),
    gravity: 0.0,
    orbital_velocity: 0.0.into(),
    radial_velocity: 0.0.into(),
    velocity_center: None,
    noise: None,
    opacity: 1.0.into(),
    progress: 0.5,
    age_spread: 0.0,
    start_speed: (0.0.into(), 0.0.into()),
    velocity_over_lifetime: (0.0.into(), 0.0.into()),
    sub_emitters: Vec::new(),
    options: None,
  }
}

impl ParticleSystem {
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

  pub fn with_size(mut self, p_size: impl Into<Range>) -> Self {
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

  pub fn with_rotation(mut self, p_degrees: impl Into<Range>) -> Self {
    self.rotation = p_degrees.into();
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

  pub fn with_hue_variation(mut self, p_degrees: impl Into<Range>) -> Self {
    self.hue_variation = p_degrees.into();
    self
  }

  pub fn with_gravity(mut self, p_gravity: f32) -> Self {
    self.gravity = p_gravity;
    self
  }

  pub fn with_orbital_velocity(mut self, p_degrees: impl Into<Range>) -> Self {
    self.orbital_velocity = p_degrees.into();
    self
  }

  pub fn with_radial_velocity(mut self, p_pixels: impl Into<Range>) -> Self {
    self.radial_velocity = p_pixels.into();
    self
  }

  pub fn with_velocity_center(mut self, p_x: f32, p_y: f32) -> Self {
    self.velocity_center = Some((p_x, p_y));
    self
  }

  pub fn with_noise(mut self, p_strength: impl Into<Range>, p_swirl_size: f32) -> Self {
    self.noise = Some((p_strength.into(), p_swirl_size.max(1.0)));
    self
  }

  pub fn with_opacity(mut self, p_opacity: impl Into<Range>) -> Self {
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

  pub fn with_start_speed(mut self, p_x: impl Into<Range>, p_y: impl Into<Range>) -> Self {
    self.start_speed = (p_x.into(), p_y.into());
    self
  }

  pub fn with_velocity_over_lifetime(mut self, p_x: impl Into<Range>, p_y: impl Into<Range>) -> Self {
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
  /// `p_particles` in the order they are drawn, a particle's children before it. A trail goes to `p_trails`.
  fn collect<'a>(
    &'a self,
    p_surface: &Surface,
    p_rng: &mut StdRng,
    p_origin: Option<(f32, f32)>,
    p_particles: &mut Vec<Particle<'a>>,
    p_trails: &mut Vec<Trail<'a>>,
  ) {
    let (width, height) = (p_surface.width, p_surface.height);
    // The pixels a luminance emitter can be born on, found once for all the particles.
    let light: Vec<usize> = match (&self.emitter, p_origin) {
      (Emitter::Luminance(min), None) => p_surface
        .source
        .unwrap_or_default()
        .chunks_exact(4)
        .enumerate()
        .filter(|(_, pixel)| (0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32) / 255.0 >= *min)
        .map(|(at, _)| at)
        .collect(),
      _ => Vec::new(),
    };
    for _ in 0..self.count {
      let (bx, by) = match (p_origin, &self.emitter) {
        (Some(origin), _) => origin,
        (None, Emitter::Image) => (p_rng.random_range(0.0..width as f32), p_rng.random_range(0.0..height as f32)),
        (None, Emitter::Circle { center, radius }) => {
          let angle = p_rng.random_range(0.0..std::f32::consts::TAU);
          let distance = radius * p_rng.random_range(0.0f32..1.0).sqrt();
          (center.0 + angle.cos() * distance, center.1 + angle.sin() * distance)
        }
        (None, Emitter::Ring { center, radius, inner_radius }) => {
          let angle = p_rng.random_range(0.0..std::f32::consts::TAU);
          let (inner, outer) = (inner_radius.min(*radius), radius.max(*inner_radius));
          let distance = (inner * inner + (outer * outer - inner * inner) * p_rng.random_range(0.0f32..1.0)).sqrt();
          (center.0 + angle.cos() * distance, center.1 + angle.sin() * distance)
        }
        (None, Emitter::Rect(left, top, w, h)) => (
          p_rng.random_range(*left..left + w.max(f32::EPSILON)),
          p_rng.random_range(*top..top + h.max(f32::EPSILON)),
        ),
        (None, Emitter::Line(from, to)) => {
          let t = p_rng.random_range(0.0f32..1.0);
          (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
        }
        (None, Emitter::Luminance(_)) if !light.is_empty() => {
          let at = light[p_rng.random_range(0..light.len())];
          ((at % width as usize) as f32 + p_rng.random_range(0.0f32..1.0), (at / width as usize) as f32 + p_rng.random_range(0.0f32..1.0))
        }
        (None, Emitter::Luminance(_)) => (p_rng.random_range(0.0..width as f32), p_rng.random_range(0.0..height as f32)),
      };
      let center = self.velocity_center.or(p_origin).unwrap_or_else(|| self.emitter.center(width, height));

      let spread = if self.age_spread > 0.0 { p_rng.random_range(-self.age_spread..self.age_spread) } else { 0.0 };
      let age = (self.progress + spread).clamp(0.0, 1.0);
      let mut pick = || p_rng.random_range(0.0f32..1.0);
      let (size_pick, opacity_pick, x_pick, y_pick) = (pick(), pick(), pick(), pick());
      let (hue_pick, orbital_pick, radial_pick, noise_pick) = (pick(), pick(), pick(), pick());
      let rotation_pick = pick();
      let (speed_x, speed_y) = (self.start_speed.0.at(0.0, pick()), self.start_speed.1.at(0.0, pick()));
      let at_age = |p_age: f32| {
        let (velocity_x, velocity_y) = &self.velocity_over_lifetime;
        let mut x = bx + speed_x * p_age + velocity_x.area(p_age, x_pick);
        let mut y = by + speed_y * p_age + velocity_y.area(p_age, y_pick);

        // Orbital velocity turns the particle around the center and radial velocity moves it along the line from it.
        let (dx, dy) = (x - center.0, y - center.1);
        let length = (dx * dx + dy * dy).sqrt();
        if length > 0.0 {
          let scale = (length + self.radial_velocity.area(p_age, radial_pick)).max(0.0) / length;
          let (sin, cos) = self.orbital_velocity.area(p_age, orbital_pick).to_radians().sin_cos();
          x = center.0 + (dx * cos - dy * sin) * scale;
          y = center.1 + (dx * sin + dy * cos) * scale;
        }

        // Gravity pulls the particle down after it has been pushed or turned, so that it does not change which way it is pushed.
        y += 0.5 * self.gravity * p_age * p_age;

        // Noise is read where the particle is, so particles near each other are pushed the same way. It grows with
        // age so a particle is born where it was meant to be.
        if let Some((strength, swirl)) = &self.noise {
          let amount = strength.at(p_age, noise_pick) * p_age;
          let (nx, ny) = (x / swirl, y / swirl);
          x += amount * value_noise(nx, ny, self.seed);
          y += amount * value_noise(nx + 31.7, ny + 17.3, self.seed);
        }
        (x, y)
      };

      // Children go down first so the parent is drawn over them.
      for (trigger, child) in &self.sub_emitters {
        if age >= *trigger {
          child.collect(p_surface, p_rng, Some(at_age(*trigger)), p_particles, p_trails);
        }
      }

      let start = self.start_color.as_ref().map(|gradient| gradient.color_at(p_rng.random_range(0.0f32..1.0)));
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
              (source[at], source[at + 1], source[at + 2], 255)
            }
            None => (0, 0, 0, 255),
          },
        };
        let (r, g, b, _) = Color::from_rgba(r, g, b, a).with_hue_rotated(self.hue_variation.at(p_age, hue_pick)).rgba();
        let (x, y) = at_age(p_age);
        let size = self.size.at(p_age, size_pick).max(0.0);
        let opacity = self.opacity.at(p_age, opacity_pick).clamp(0.0, 1.0) * a as f32 / 255.0 * p_fade;
        // A texture is a rectangle that can turn, so it reaches as far as its corners; a soft dot fades out at one and a half times its size.
        let reach = (if self.texture.is_some() { size * std::f32::consts::SQRT_2 } else { size * 1.5 }).ceil() as i64 + 1;
        let (sin, cos) = self.rotation.at(p_age, rotation_pick).to_radians().sin_cos();
        Particle { system: self, x, y, size, opacity, color: (r, g, b), sin, cos, reach, birth }
      };

      // The streak is the particle at earlier ages, as a line from its tail to its head. Each sample says how wide, how opaque and
      // what color the line is there. A line too short to be seen is left out.
      let head = at(age, 1.0);
      let span = self.trail.min(age);
      if span > 0.0 {
        const SAMPLES: usize = 24;
        let samples: Vec<TrailSample> = (0..=SAMPLES)
          .map(|i| {
            let behind = 1.0 - i as f32 / SAMPLES as f32;
            let sample = at(age - span * behind, 1.0 - self.trail_fade * behind);
            TrailSample { x: sample.x, y: sample.y, width: sample.size * 2.0, opacity: sample.opacity, color: sample.color }
          })
          .collect();
        let length: f32 = samples.windows(2).map(|pair| ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt()).sum();
        if length >= 0.5 {
          p_trails.push(Trail { system: self, samples });
        }
      }
      p_particles.push(head);
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
    let source = p_image.to_rgba_vec();
    let (mut particles, mut trails) = (Vec::new(), Vec::new());
    let mut rng = StdRng::seed_from_u64(self.seed);
    self.collect(&Surface { source: Some(&source), width, height }, &mut rng, None, &mut particles, &mut trails);

    if !trails.is_empty() {
      let mut painter = Painter::new(p_image);
      for trail in &trails {
        let (path, brush) = trail.stroke();
        painter.stroke_with_brush(&path, &brush);
      }
    }
    let mut pixels = p_image.to_rgba_vec();
    draw(&particles, &mut pixels, width, height);
    p_image.set_rgba(pixels);
  }
}

/// Draws the particles onto `p_pixels`. The image is cut into bands of rows, and each band is drawn by one thread with the
/// particles that touch it, in the order they were made, so overlaps come out the same as drawing them one at a time.
fn draw(p_particles: &[Particle], p_pixels: &mut [u8], p_width: u32, p_height: u32) {
  if p_particles.is_empty() || p_width == 0 || p_height == 0 {
    return;
  }
  let rows = (p_height as usize).div_ceil(rayon::current_num_threads() * 4).max(1);
  let mut bands: Vec<Vec<u32>> = vec![Vec::new(); (p_height as usize).div_ceil(rows)];
  for (index, particle) in p_particles.iter().enumerate() {
    let top = (particle.y as i64).saturating_sub(particle.reach).max(0);
    let bottom = (particle.y as i64).saturating_add(particle.reach).min(p_height as i64 - 1);
    if top <= bottom {
      for band in top as usize / rows..=bottom as usize / rows {
        bands[band].push(index as u32);
      }
    }
  }
  p_pixels.par_chunks_mut(rows * p_width as usize * 4).zip(bands.par_iter()).enumerate().for_each(|(band, (chunk, list))| {
    for &index in list {
      p_particles[index as usize].draw(chunk, band * rows, p_width, p_height);
    }
  });
}

impl Particle<'_> {
  /// Draws the particle onto the rows of the image in `p_band`, which starts at row `p_first_row`.
  fn draw(&self, p_band: &mut [u8], p_first_row: usize, p_width: u32, p_height: u32) {
    let system = self.system;
    let texture = system.texture.as_ref().map(|texture| (texture.rgba(), texture.dimensions::<u32>()));
    let (x, y, size, opacity, (r, g, b)) = (self.x, self.y, self.size, self.opacity, self.color);
    let last_row = (p_first_row + p_band.len() / (p_width as usize * 4)).min(p_height as usize) as i64 - 1;
    let top = (y as i64).saturating_sub(self.reach).max(p_first_row as i64);
    let bottom = (y as i64).saturating_add(self.reach).min(last_row);
    for py in top..=bottom {
      for px in (x as i64).saturating_sub(self.reach).max(0)..=(x as i64).saturating_add(self.reach).min(p_width as i64 - 1) {
        let (dx, dy) = (px as f32 + 0.5 - x, py as f32 + 0.5 - y);
        let (r, g, b, coverage) = match texture {
          None => (r, g, b, ((size - (dx * dx + dy * dy).sqrt()) / size.max(1.0) + 0.5).clamp(0.0, 1.0) * opacity),
          Some((pixels, (texture_width, texture_height))) => {
            // Turn the pixel back by the particle's rotation to find where it falls on the texture.
            let (u, v) = (dx * self.cos + dy * self.sin, -dx * self.sin + dy * self.cos);
            let longest = texture_width.max(texture_height) as f32;
            let (half_width, half_height) = (size * texture_width as f32 / longest, size * texture_height as f32 / longest);
            let (tu, tv) = (u / (2.0 * half_width) + 0.5, v / (2.0 * half_height) + 0.5);
            if !(0.0..1.0).contains(&tu) || !(0.0..1.0).contains(&tv) {
              continue;
            }
            let sample = texel(pixels, texture_width, texture_height, tu, tv);
            let tint = |texture: f32, particle: u8| (texture * particle as f32 / 255.0) as u8;
            (tint(sample[0], r), tint(sample[1], g), tint(sample[2], b), sample[3] / 255.0 * opacity)
          }
        };
        if coverage <= 0.0 {
          continue;
        }
        let at = ((py as usize - p_first_row) * p_width as usize + px as usize) * 4;
        let (new_r, new_g, new_b, new_a) =
          system.blend.composite(r, g, b, 255, coverage, p_band[at], p_band[at + 1], p_band[at + 2], p_band[at + 3]);
        p_band[at..at + 4].copy_from_slice(&[new_r, new_g, new_b, new_a]);
      }
    }
  }
}

/// Smooth noise from -1.0 to 1.0 that changes slowly across `p_x` and `p_y`: a random value at each whole-number
/// point, blended between them.
fn value_noise(p_x: f32, p_y: f32, p_seed: u64) -> f32 {
  let corner = |ix: i32, iy: i32| {
    let mut h = (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (iy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ p_seed;
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    (h >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
  };
  let (ix, iy) = (p_x.floor(), p_y.floor());
  let ease = |t: f32| t * t * (3.0 - 2.0 * t);
  let (tx, ty) = (ease(p_x - ix), ease(p_y - iy));
  let (ix, iy) = (ix as i32, iy as i32);
  let top = corner(ix, iy) + (corner(ix + 1, iy) - corner(ix, iy)) * tx;
  let bottom = corner(ix, iy + 1) + (corner(ix + 1, iy + 1) - corner(ix, iy + 1)) * tx;
  top + (bottom - top) * ty
}

/// The color of a texture at `p_u`, `p_v` (0.0 to 1.0 across it), mixed from the four pixels around that spot.
fn texel(p_pixels: &[u8], p_width: u32, p_height: u32, p_u: f32, p_v: f32) -> [f32; 4] {
  let x = (p_u * p_width as f32 - 0.5).clamp(0.0, (p_width - 1) as f32);
  let y = (p_v * p_height as f32 - 0.5).clamp(0.0, (p_height - 1) as f32);
  let (x0, y0) = (x.floor() as usize, y.floor() as usize);
  let (x1, y1) = ((x0 + 1).min(p_width as usize - 1), (y0 + 1).min(p_height as usize - 1));
  let (tx, ty) = (x - x0 as f32, y - y0 as f32);
  let at = |x: usize, y: usize, channel: usize| p_pixels[(y * p_width as usize + x) * 4 + channel] as f32;
  let mut color = [0.0; 4];
  for (channel, value) in color.iter_mut().enumerate() {
    let top = at(x0, y0, channel) + (at(x1, y0, channel) - at(x0, y0, channel)) * tx;
    let bottom = at(x0, y1, channel) + (at(x1, y1, channel) - at(x0, y1, channel)) * tx;
    *value = top + (bottom - top) * ty;
  }
  color
}

/// How many words wide the texture that carries the particles to the GPU is. A word is 32 bits and a texel holds one.
const AUX_WIDTH: u32 = 4096;
/// The most rows that texture may have, which is the usual largest side of a texture.
const AUX_ROWS: u32 = 8192;
/// The most words a system may be worked out to need and still go to the GPU. Counting the lists of what touches each tile, it stays
/// inside the texture.
const MAX_WORDS: usize = 24_000_000;
/// The side, in pixels, of the squares the image is cut into for the GPU, each with the list of what touches it.
const TILE: u32 = 16;
/// The blend word of a particle that adds light, which is no mode of the blend shader.
const ADDITIVE: u32 = 0xFFFF;
/// Set in the blend word of a particle whose color is the picture's at its birth point.
const FROM_PIXEL: u32 = 0x1_0000;
/// The sprite word of a particle that is a dot.
const NO_SPRITE: u32 = u32::MAX;
/// Set in an item of a tile's list that is a trail and not a particle.
const TRAIL_ITEM: u32 = 0x8000_0000;

impl ParticleSystem {
  /// Whether the GPU can draw the system: what the shader cannot do is a luminance emitter, which picks pixels of the picture that
  /// the particles are made without, a color taken from the picture that is turned in hue or put in a trail, a blend mode that has
  /// no shader, and more particles than a texture holds. Those are drawn by the CPU.
  fn gpu_supported(&self) -> bool {
    self.gpu_supported_as(false) && self.gpu_words() <= MAX_WORDS
  }

  fn gpu_supported_as(&self, p_child: bool) -> bool {
    let from_picture = self.start_color.is_none() && self.color_over_lifetime.is_none() && self.texture.is_none();
    let emitter = p_child || !matches!(self.emitter, Emitter::Luminance(_));
    let colors = !from_picture || (self.hue_variation.is_zero() && self.trail <= 0.0);
    let blend = match self.blend {
      Blend::Additive => true,
      Blend::Mode(mode) => mode.shader_index().is_some(),
    };
    emitter && colors && blend && self.sub_emitters.iter().all(|(_, child)| child.gpu_supported_as(true))
  }

  /// The most words the system's particles, trails and textures take, worked out from the settings without making the particles.
  fn gpu_words(&self) -> usize {
    fn per_particle(p_system: &ParticleSystem) -> usize {
      // A particle is 16 words, and its trail has a line of 8 words through 25 points of 8 words each.
      let trail = if p_system.trail > 0.0 { 8 + 25 * 8 } else { 0 };
      let children: usize = p_system.sub_emitters.iter().map(|(_, child)| child.count.saturating_mul(per_particle(child))).sum();
      16 + trail + children
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
    self.count.saturating_mul(per_particle(self)).saturating_add(seen.iter().map(|(_, words)| words).sum::<usize>())
  }
}

/// The particles are made on the CPU, which has to, since they share a random stream, and the shader draws them. It is handed them as
/// a list of words, with the image cut into squares and the particles and trails that touch each one in the order they are drawn.
impl GpuProcessor for ParticleSystem {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let (mut particles, mut trails) = (Vec::new(), Vec::new());
    let mut rng = StdRng::seed_from_u64(self.seed);
    self.collect(&Surface { source: None, width: p_width, height: p_height }, &mut rng, None, &mut particles, &mut trails);
    vec![gpu_pass(&particles, &trails, p_width, p_height)]
  }
}

/// The shader, joined after the blend modes it uses. It is made once, so the renderer can tell it is the same shader every time.
fn gpu_shader() -> &'static str {
  static SHADER: OnceLock<String> = OnceLock::new();
  SHADER.get_or_init(|| format!("{BLEND_MODES_WGSL}{}", include_str!("./particles.wgsl"))).as_str()
}

/// The number the shader gives a blend: its mode's index, or `ADDITIVE`.
fn blend_code(p_blend: Blend) -> u32 {
  match p_blend {
    Blend::Additive => ADDITIVE,
    Blend::Mode(mode) => mode.shader_index().expect("a particle system with a mode that has no shader is not drawn on the GPU"),
  }
}

/// Calls `p_each` with the number of every tile that the pixels from `p_left` to `p_right` and `p_top` to `p_bottom` touch.
fn for_each_tile(p_left: i64, p_top: i64, p_right: i64, p_bottom: i64, p_width: u32, p_height: u32, mut p_each: impl FnMut(usize)) {
  let (left, top) = (p_left.max(0), p_top.max(0));
  let (right, bottom) = (p_right.min(p_width as i64 - 1), p_bottom.min(p_height as i64 - 1));
  if left > right || top > bottom {
    return;
  }
  let tiles_x = p_width.div_ceil(TILE) as i64;
  for tile_y in top / TILE as i64..=bottom / TILE as i64 {
    for tile_x in left / TILE as i64..=right / TILE as i64 {
      p_each((tile_y * tiles_x + tile_x) as usize);
    }
  }
}

/// The pass that draws `p_particles` and `p_trails` on an image of `p_width` x `p_height` pixels. See `particles.wgsl` for what the
/// words are.
fn gpu_pass(p_particles: &[Particle], p_trails: &[Trail], p_width: u32, p_height: u32) -> GpuPass {
  // Each texture is written once, however many particles use it: its image, where its pixels start among the texture words, and its size.
  let mut textures: Vec<(Image, u32, u32, u32)> = Vec::new();
  let mut texture_words = 0u32;
  let sprites: Vec<Option<(u32, u32, u32)>> = p_particles
    .iter()
    .map(|particle| {
      let image = particle.system.texture.as_ref()?;
      if let Some((_, start, width, height)) = textures.iter().find(|(known, ..)| known.version() == image.version()) {
        return Some((*start, *width, *height));
      }
      let (width, height) = image.dimensions::<u32>();
      textures.push((image.clone(), texture_words, width, height));
      texture_words += width * height;
      Some((texture_words - width * height, width, height))
    })
    .collect();

  let vertex_words: usize = p_trails.iter().map(|trail| trail.samples.len() * 8).sum();
  let trails_at = (p_particles.len() * 16) as u32;
  let vertices_at = trails_at + (p_trails.len() * 8) as u32;
  let textures_at = vertices_at + vertex_words as u32;
  let tiles_at = textures_at + texture_words;
  let (tiles_x, tiles_y) = (p_width.div_ceil(TILE), p_height.div_ceil(TILE));
  let list_at = tiles_at + tiles_x * tiles_y + 1;

  let mut words: Vec<u32> = Vec::with_capacity(list_at as usize);
  for (particle, sprite) in p_particles.iter().zip(&sprites) {
    let (sprite_at, sprite_width, sprite_height) = match sprite {
      Some((start, width, height)) => (textures_at + start, *width, *height),
      None => (NO_SPRITE, 0, 0),
    };
    let mut blend = blend_code(particle.system.blend);
    let (birth_x, birth_y) = match particle.birth {
      Some(born) => {
        blend |= FROM_PIXEL;
        born
      }
      None => (0.0, 0.0),
    };
    words.extend([
      particle.x.to_bits(),
      particle.y.to_bits(),
      particle.size.to_bits(),
      particle.opacity.to_bits(),
      (particle.color.0 as f32).to_bits(),
      (particle.color.1 as f32).to_bits(),
      (particle.color.2 as f32).to_bits(),
      particle.sin.to_bits(),
      particle.cos.to_bits(),
      particle.reach.clamp(0, i32::MAX as i64) as i32 as u32,
      blend,
      sprite_at,
      sprite_width,
      sprite_height,
      birth_x.to_bits(),
      birth_y.to_bits(),
    ]);
  }
  let mut first_point = 0u32;
  for trail in p_trails {
    let points = trail.samples.len() as u32;
    words.extend([first_point, points, trail.system.trail_hardness.to_bits(), blend_code(trail.system.blend), 0, 0, 0, 0]);
    first_point += points;
  }
  for trail in p_trails {
    for (sample, time) in trail.samples.iter().zip(trail.times()) {
      words.extend([
        sample.x.to_bits(),
        sample.y.to_bits(),
        (sample.width / 2.0).to_bits(),
        sample.opacity.to_bits(),
        (sample.color.0 as f32).to_bits(),
        (sample.color.1 as f32).to_bits(),
        (sample.color.2 as f32).to_bits(),
        time.to_bits(),
      ]);
    }
  }
  for (image, ..) in &textures {
    words.extend(image.rgba().chunks_exact(4).map(|pixel| u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]])));
  }

  // The trails are drawn first, under every particle, so they are listed first in each tile.
  let mut tiles: Vec<Vec<u32>> = vec![Vec::new(); (tiles_x * tiles_y) as usize];
  for (index, trail) in p_trails.iter().enumerate() {
    let mut touched: Vec<usize> = Vec::new();
    for pair in trail.samples.windows(2) {
      let reach = pair[0].width.max(pair[1].width) / 2.0 + 1.5;
      let (left, right) = (pair[0].x.min(pair[1].x) - reach, pair[0].x.max(pair[1].x) + reach);
      let (top, bottom) = (pair[0].y.min(pair[1].y) - reach, pair[0].y.max(pair[1].y) + reach);
      for_each_tile(left.floor() as i64, top.floor() as i64, right.ceil() as i64, bottom.ceil() as i64, p_width, p_height, |tile| touched.push(tile));
    }
    touched.sort_unstable();
    touched.dedup();
    for tile in touched {
      tiles[tile].push(TRAIL_ITEM | index as u32);
    }
  }
  for (index, particle) in p_particles.iter().enumerate() {
    let (x, y) = (particle.x as i64, particle.y as i64);
    let reach = particle.reach;
    for_each_tile(x.saturating_sub(reach), y.saturating_sub(reach), x.saturating_add(reach), y.saturating_add(reach), p_width, p_height, |tile| {
      tiles[tile].push(index as u32)
    });
  }
  let mut offset = 0u32;
  for tile in &tiles {
    words.push(offset);
    offset += tile.len() as u32;
  }
  words.push(offset);
  for tile in &tiles {
    words.extend(tile);
  }

  let rows = (words.len() as u32).div_ceil(AUX_WIDTH).clamp(1, AUX_ROWS);
  words.resize((rows * AUX_WIDTH) as usize, 0);
  let rgba: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
  let uniforms: Vec<u8> = [AUX_WIDTH, 0, trails_at, vertices_at, tiles_at, list_at, TILE, tiles_x, 0, 0, 0, 0]
    .iter()
    .flat_map(|word| word.to_le_bytes())
    .collect();
  GpuPass::new(gpu_shader(), uniforms).with_aux(GpuAux { width: AUX_WIDTH, height: rows, rgba: Arc::from(rgba) }).with_pixel_exact()
}
