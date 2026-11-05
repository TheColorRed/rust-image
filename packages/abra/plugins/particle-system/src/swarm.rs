use abra::abra_core::{CurveRange, Image};
use abra::animation::prelude::{Ensemble, EnsembleOptions, HISTORY_POINT_VALUES, MEMBER_VALUES, Motion, POSE_VALUES, PoseNoise, Schedule, SpeedResponse};
use abra::drawing::prelude::{Blend, Sprites, sprites};

/// How the particles of one swarm are drawn, besides where they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwarmStyle {
  /// How hard the edge of a dot is, from 0.0 (soft) to 1.0 (hard).
  pub hardness: f32,
  /// How hard the edge of a trail is, from 0.0 (a soft fade) to 1.0 (a hard edge).
  pub trail_hardness: f32,
  /// Whether a particle's color is read from the picture where it was born, so it is left white here and its birth point is listed with it.
  pub picture_color: bool,
  /// How many frames a texture is cut into, `columns` across and `rows` down. A texture of one frame is `(1, 1)`.
  pub frames: (u32, u32),
}

impl Default for SwarmStyle {
  fn default() -> Self {
    SwarmStyle { hardness: 0.0, trail_hardness: 0.5, picture_color: false, frames: (1, 1) }
  }
}

/// How many numbers describe one particle in [`Swarm::new`] (see `animation::MEMBER_VALUES`).
pub const PARTICLE_VALUES: usize = MEMBER_VALUES;

/// Many particles that live and die together, made once and drawn at any moment of their life in one call: [`Swarm::sprites`] works out how every
/// particle looks (with the `animation` crate) and hands them to the drawing library as the sprites and trails to draw. A swarm can have swarms
/// born from its particles. Each swarm has a group, a whole number, which says which texture its particles are drawn as.
#[derive(Clone, Debug)]
pub struct Swarm {
  ensemble: Ensemble,
  /// The style of each group in the swarm and in the swarms born from it.
  styles: Vec<(u32, SwarmStyle)>,
}

impl Swarm {
  /// A swarm of the particles in `p_values`, [`PARTICLE_VALUES`] numbers for each (see `animation::Ensemble::new`), that move as `p_motion` says. The
  /// size, opacity, rotation and hue are ranges of the life, read with each particle's pick. `p_looping` says whether the progress goes round and
  /// round, `p_trail` how far back along its path a particle leaves a streak (as a share of its life, 0 for none) and `p_trail_fade` how much the
  /// streak fades toward its tail.
  ///
  /// # Panics
  /// When the length of `p_values` is not a whole number of particles.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    p_motion: Motion, p_values: &[f32], p_looping: bool, p_trail: f32, p_trail_fade: f32, p_group: u32, p_style: SwarmStyle, p_size: CurveRange,
    p_opacity: CurveRange, p_rotation: CurveRange, p_hue: CurveRange,
  ) -> Swarm {
    let options = EnsembleOptions { looping: p_looping, history: p_trail, history_fade: p_trail_fade, tag: p_group };
    Swarm {
      ensemble: Ensemble::new(p_motion, p_values, options, p_size, p_opacity, p_rotation, p_hue),
      styles: vec![(p_group, p_style)],
    }
  }

  /// The color the particles are multiplied by over a life (see `animation::Ensemble::with_lifetime_colors`).
  pub fn with_lifetime_colors(mut self, p_colors: &[f32]) -> Swarm {
    self.ensemble = self.ensemble.with_lifetime_colors(p_colors);
    self
  }

  /// Makes the size, rotation and color of the particles follow how fast they go (see `animation::SpeedResponse`).
  pub fn with_speed_response(mut self, p_response: SpeedResponse) -> Swarm {
    self.ensemble = self.ensemble.with_speed_response(p_response);
    self
  }

  /// Noise that changes the size and the rotation of the particles where they are (see `animation::PoseNoise`).
  pub fn with_pose_noise(mut self, p_noise: PoseNoise) -> Swarm {
    self.ensemble = self.ensemble.with_pose_noise(p_noise);
    self
  }

  /// Stretches each particle along the way it goes, and turns it that way (see `animation::Ensemble::with_velocity_stretch`).
  pub fn with_velocity_stretch(mut self, p_scale: f32) -> Swarm {
    self.ensemble = self.ensemble.with_velocity_stretch(p_scale);
    self
  }

  /// Treats the texture of this swarm as a grid of `p_columns` x `p_rows` frames, played along each particle's life `p_cycles` times.
  pub fn with_frames(mut self, p_columns: u32, p_rows: u32, p_cycles: f32) -> Swarm {
    let (columns, rows) = (p_columns.max(1), p_rows.max(1));
    self.ensemble = self.ensemble.with_frames(columns * rows, p_cycles);
    if let Some((_, style)) = self.styles.first_mut() {
      style.frames = (columns, rows);
    }
    self
  }

  /// Makes the particles last over time (see `animation::Ensemble::with_schedule`): from then on `p_progress` of [`Swarm::sprites`] is the time in
  /// seconds.
  pub fn with_schedule(mut self, p_schedule: Schedule, p_lifetime_picks: &[f32]) -> Swarm {
    self.ensemble = self.ensemble.with_schedule(p_schedule, p_lifetime_picks);
    self
  }

  /// A swarm born from the particles of this one (see `animation::Ensemble::with_child`).
  pub fn with_child(mut self, p_trigger: f32, p_child: Swarm, p_counts: &[u32]) -> Swarm {
    self.styles.extend(p_child.styles);
    self.ensemble = self.ensemble.with_child(p_trigger, p_child.ensemble, p_counts);
    self
  }

  /// How many particles there are, not counting the ones born from them.
  pub fn len(&self) -> usize {
    self.ensemble.len()
  }

  pub fn is_empty(&self) -> bool {
    self.ensemble.is_empty()
  }

  fn style(&self, p_group: u32) -> SwarmStyle {
    self.styles.iter().find(|(group, _)| *group == p_group).map(|(_, style)| *style).unwrap_or_default()
  }

  /// The sprites and trails of the swarm `p_progress` (0 to 1) of the way through its life, or at that many seconds when it has an schedule, to
  /// draw with `p_blend`. `p_textures` are the pictures the
  /// groups are drawn as, by group number (a missing one, or `None`, is a soft dot).
  pub fn sprites(&self, p_progress: f32, p_blend: Blend, p_textures: &[Option<Image>]) -> Sprites {
    let frame = self.ensemble.evaluate(p_progress);
    let mut batch = sprites();
    for run in &frame.histories {
      // The streak is as wide as the particle is across.
      let mut values = Vec::with_capacity(run.values.len());
      for point in run.values.chunks_exact(HISTORY_POINT_VALUES) {
        values.extend([point[0], point[1], point[2] * 2.0, point[3], point[4], point[5], point[6]]);
      }
      batch = batch.add_trail_values(&values, &run.lengths, self.style(run.tag).trail_hardness, p_blend);
    }
    for run in &frame.poses {
      let style = self.style(run.tag);
      let mut values = Vec::with_capacity(run.values.len() / POSE_VALUES * 16);
      for pose in run.values.chunks_exact(POSE_VALUES) {
        let (born_x, born_y) = if style.picture_color { (pose[8], pose[9]) } else { (0.0, 0.0) };
        values.extend([
          pose[0],
          pose[1],
          pose[2],
          pose[3],
          pose[4],
          pose[5],
          pose[6],
          style.hardness,
          pose[7],
          if style.picture_color { 1.0 } else { 0.0 },
          born_x,
          born_y,
          pose[10],
          style.frames.0 as f32,
          style.frames.1 as f32,
          pose[11],
        ]);
      }
      let texture = p_textures.get(run.tag as usize).and_then(|texture| texture.clone());
      batch = batch.add_sprite_values(&values, p_blend, texture);
    }
    batch
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A particle's numbers: born at `x`, 10, no motion of its own, no start color.
  fn particle(p_x: f32) -> Vec<f32> {
    let mut values = vec![p_x, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    values.extend([0.0, 0.0, 0.5, 0.5, 0.5, 0.5]);
    values.extend([0.0, 0.0, 0.0, 0.0, 0.0]);
    values
  }

  fn swarm(p_values: &[f32], p_trail: f32, p_group: u32, p_style: SwarmStyle) -> Swarm {
    let moving = Motion::new().with_velocity(40.0, 0.0);
    Swarm::new(moving, p_values, false, p_trail, 1.0, p_group, p_style, 4.0.into(), 0.5.into(), 0.0.into(), 0.0.into())
  }

  fn picture() -> Image {
    let mut image = Image::new(32, 32);
    image.set_rgba(vec![0u8; 32 * 32 * 4]);
    image
  }

  #[test]
  fn a_swarm_draws_stretch_and_the_frame_of_a_texture_as_the_system_does() {
    use abra::options::prelude::Effect;
    let mut sheet = Image::new(8, 4);
    let pixels: Vec<u8> = (0..8 * 4).flat_map(|i| if i % 8 < 4 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }).collect();
    sheet.set_rgba(pixels);
    let thrown = |age: f32| {
      let moving = Motion::new().with_velocity(40.0, 0.0);
      let swarm = Swarm::new(moving, &particle(8.0), false, 0.0, 1.0, 0, SwarmStyle::default(), 4.0.into(), 1.0.into(), 0.0.into(), 0.0.into())
        .with_velocity_stretch(0.05)
        .with_frames(2, 1, 1.0);
      let mut image = picture();
      swarm.sprites(age, Blend::default(), &[Some(sheet.clone())]).apply_on_cpu(&mut image);
      image
    };
    let (early, late) = (thrown(0.1), thrown(0.9));
    assert_ne!(early.rgba(), late.rgba(), "a different frame, in a different place");
    let lit: Vec<(u32, u32)> = (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).filter(|(x, y)| early.get_pixel(*x, *y).unwrap().3 > 0).collect();
    assert!(lit.iter().map(|p| p.0).max().unwrap() - lit.iter().map(|p| p.0).min().unwrap() > 6, "stretched along its way");
  }

  #[test]
  fn a_swarm_is_drawn_as_the_sprites_and_trails_of_the_objects() {
    use abra::drawing::prelude::{Sprite, SpriteTrail, TrailPoint};
    use abra::options::prelude::Effect;
    let style = SwarmStyle { hardness: 0.25, trail_hardness: 0.8, ..SwarmStyle::default() };
    let batch = swarm(&particle(2.0), 0.5, 0, style).sprites(0.5, Blend::default(), &[]);
    let mut by_numbers = picture();
    batch.apply_on_cpu(&mut by_numbers);

    // The same particle by hand: it is at 2 + 40 * age after a life of `age`, and its streak reaches back over half the life.
    let age = 0.5f32;
    let points: Vec<TrailPoint> = (0..=24)
      .map(|i| {
        let behind = 1.0 - i as f32 / 24.0;
        let at = age - age.min(0.5) * behind;
        TrailPoint::new(2.0 + 40.0 * at, 10.0, 8.0).with_opacity(0.5 * (1.0 - behind))
      })
      .collect();
    let by_hand = abra::drawing::prelude::sprites()
      .with_trails(vec![SpriteTrail::new(points).with_hardness(0.8)])
      .with_sprites(vec![Sprite::new(2.0 + 40.0 * age, 10.0, 4.0).with_opacity(0.5).with_hardness(0.25).with_rotation(0.0)]);
    let mut expected = picture();
    by_hand.apply_on_cpu(&mut expected);
    assert_eq!(by_numbers.rgba(), expected.rgba());
  }

  #[test]
  fn with_an_emission_only_the_particles_alive_at_that_time_are_drawn() {
    use abra::options::prelude::Effect;
    let both = [particle(2.0), particle(4.0)].concat();
    let schedule = Schedule::new(5.0).with_burst(0.0, 1).with_burst(2.0, 1).with_lifetime(1.0, 1.0);
    let timed = swarm(&both, 0.0, 0, SwarmStyle { hardness: 1.0, ..SwarmStyle::default() }).with_schedule(schedule, &[0.0, 0.0]);
    let drawn = |time: f32| {
      let mut image = picture();
      timed.sprites(time, Blend::default(), &[]).apply_on_cpu(&mut image);
      image.rgba().iter().any(|value| *value != 0)
    };
    assert!(drawn(0.5), "the first is alive at half a second");
    assert!(!drawn(1.5), "and nothing is alive between the two");
    assert!(drawn(2.5), "the second is alive at two and a half");
  }

  #[test]
  fn a_swarm_born_from_another_keeps_its_own_style() {
    let parent = swarm(&particle(1.0), 0.0, 0, SwarmStyle::default());
    let child = swarm(&particle(2.0), 0.0, 1, SwarmStyle { hardness: 1.0, ..SwarmStyle::default() });
    let both = parent.with_child(0.0, child, &[1]);
    assert_eq!(both.style(1).hardness, 1.0);
    assert_eq!(both.style(0).hardness, 0.0);
    assert_eq!(both.style(7), SwarmStyle::default());
  }
}
