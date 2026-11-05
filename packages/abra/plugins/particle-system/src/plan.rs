//! Makes a [`Swarm`] from a [`ParticleSystem`]: where every particle is born and what it picks at random are worked out here, once, and the swarm
//! then says how they all look at any moment of their life in one call. It is the same scatter as drawing the system itself, so an app that wants to
//! draw one system over and over (a live preview) does not work out the particles again for each frame.

use super::*;
use abra::animation::prelude::LIFETIME_STEPS;

/// What a particle is that does not change as the system goes through its life.
struct Plan {
  born: (f32, f32),
  center: (f32, f32),
  /// How far from the system's progress the particle's age is, and where it falls in a looping cycle (0.0 to 1.0).
  spread: f32,
  phase: f32,
  picks: Picks,
  /// Where its lifetime falls in the range of the schedule.
  lifetime_pick: f32,
  /// The color it started as, or nothing when it has none.
  start: Option<[u8; 4]>,
  /// For each system born from its particles, those particles' plans: they are born where this particle is at the age that triggers them.
  children: Vec<Vec<Plan>>,
}

impl ParticleSystem {
  /// The number of particles the system makes of its own: the things of its schedule, or `count`.
  fn particle_count(&self, p_origin: Option<(f32, f32)>) -> usize {
    match (&self.emission, p_origin) {
      (Some(schedule), None) => schedule.count(),
      _ => self.count,
    }
  }

  /// The plans of the particles born in the emitter, or at `p_origin` for a sub-emitter. Sub-emitters are planned for every particle and not only for
  /// those old enough to make them, so what a particle gets does not depend on how far along the system is.
  fn plan(&self, p_surface: &Surface, p_rng: &mut Random, p_origin: Option<(f32, f32)>) -> Vec<Plan> {
    let count = self.particle_count(p_origin);
    let mut light = match (&self.emitter, p_origin, p_surface.source) {
      (Emitter::Luminance(min), None, Some(source)) => p_rng.points_in_light(source, *min, count).into_iter(),
      _ => Vec::new().into_iter(),
    };
    let motion = self.motion();
    let scheduled = self.emission.is_some() && p_origin.is_none();
    (0..count)
      .map(|_| {
        let born = self.birth_point(p_surface, p_rng, p_origin, &mut light);
        let center = self.velocity_center.or(p_origin).unwrap_or_else(|| self.emitter.center(p_surface.width, p_surface.height));
        let spread = if self.age_spread > 0.0 { p_rng.range(-self.age_spread, self.age_spread) } else { 0.0 };
        let phase = if self.age_spread > 0.0 { (spread + self.age_spread) / (2.0 * self.age_spread) } else { 0.0 };
        let picks = self.draw_picks(p_rng, born, center);
        let lifetime_pick = if scheduled { p_rng.float() } else { 0.0 };
        let mover = Mover {
          born,
          center,
          speed: picks.speed,
          picks: MoverPicks { velocity_x: picks.x, velocity_y: picks.y, orbital: picks.orbital, radial: picks.radial, noise: picks.noise },
        };
        let children = self.sub_emitters.iter().map(|(trigger, child)| child.plan(p_surface, p_rng, Some(motion.at(&mover, *trigger)))).collect();
        let start = self.start_color.as_ref().map(|gradient| {
          let (r, g, b, a) = gradient.color_at(p_rng.float()).rgba();
          [r, g, b, a]
        });
        Plan { born, center, spread, phase, picks, lifetime_pick, start, children }
      })
      .collect()
  }

  /// The swarm of the particles of a system and of the systems born from them, made from their plans. The pictures the groups are drawn as are added
  /// to `p_textures` in the order of the groups, by the group number the swarm is told.
  fn swarm_of(&self, p_plans: &[&Plan], p_root: bool, p_textures: &mut Vec<Option<Image>>) -> Swarm {
    let group = p_textures.len() as u32;
    p_textures.push(self.texture.clone());
    let from_picture = self.start_color.is_none() && self.color_over_lifetime.is_none() && self.texture.is_none();
    // The renderer reads a picture's color at the birth point, so it cannot also be turned in hue or put in a trail.
    let picture_color = from_picture && self.hue_variation.is_zero() && self.trail <= 0.0;
    let style = SwarmStyle { hardness: self.hardness, trail_hardness: self.trail_hardness, picture_color, frames: self.frames };
    let values: Vec<f32> = p_plans
      .iter()
      .flat_map(|plan| {
        let start = plan.start.map_or([0.0; 5], |color| [1.0, color[0] as f32, color[1] as f32, color[2] as f32, color[3] as f32]);
        let numbers = [
          plan.born.0,
          plan.born.1,
          plan.center.0,
          plan.center.1,
          plan.picks.speed.0,
          plan.picks.speed.1,
          plan.picks.x,
          plan.picks.y,
          plan.picks.orbital,
          plan.picks.radial,
          plan.picks.noise,
          plan.phase,
          plan.spread,
          plan.picks.size,
          plan.picks.opacity,
          plan.picks.hue,
          plan.picks.rotation,
        ];
        numbers.into_iter().chain(start)
      })
      .collect();
    let mut swarm = Swarm::new(
      self.motion(),
      &values,
      self.looping,
      self.trail,
      self.trail_fade,
      group,
      style,
      self.size.clone(),
      self.opacity.clone(),
      self.rotation.clone(),
      self.hue_variation.clone(),
    );
    if let Some(gradient) = &self.color_over_lifetime {
      let colors: Vec<f32> = (0..=LIFETIME_STEPS)
        .flat_map(|step| {
          let (r, g, b, a) = gradient.color_at(step as f32 / LIFETIME_STEPS as f32).rgba();
          [r as f32, g as f32, b as f32, a as f32]
        })
        .collect();
      swarm = swarm.with_lifetime_colors(&colors);
    }
    if let (Some(schedule), true) = (&self.emission, p_root) {
      let picks: Vec<f32> = p_plans.iter().map(|plan| plan.lifetime_pick).collect();
      swarm = swarm.with_schedule(schedule.clone(), &picks);
    }
    if let Some(response) = &self.speed_response {
      swarm = swarm.with_speed_response(response.clone());
    }
    if let Some(noise) = self.pose_noise {
      swarm = swarm.with_pose_noise(noise);
    }
    if self.velocity_stretch > 0.0 {
      swarm = swarm.with_velocity_stretch(self.velocity_stretch);
    }
    if self.frames.0 * self.frames.1 > 1 {
      swarm = swarm.with_frames(self.frames.0, self.frames.1, self.frame_cycles);
    }
    for (at, (trigger, child)) in self.sub_emitters.iter().enumerate() {
      let born_from: Vec<&Plan> = p_plans.iter().flat_map(|plan| plan.children[at].iter()).collect();
      let counts: Vec<u32> = p_plans.iter().map(|plan| plan.children[at].len() as u32).collect();
      let born = child.swarm_of(&born_from, false, p_textures);
      swarm = swarm.with_child(*trigger, born, &counts);
    }
    swarm
  }

  /// The pictures the groups of [`ParticleSystem::swarm`] are drawn as, by group number: the system's own texture first, then those of the systems born
  /// from it, depth first. A group without a texture is `None`, a soft dot.
  pub fn textures(&self) -> Vec<Option<Image>> {
    let mut textures = vec![self.texture.clone()];
    for (_, child) in &self.sub_emitters {
      textures.extend(child.textures());
    }
    textures
  }

  /// A swarm of the system's particles for an image of `p_width` x `p_height` pixels, and the pictures its groups are drawn as, by group number (the
  /// system is group 0, and the systems born from it follow in order, depth first), to give to [`Swarm::sprites`]. `p_photo` is the picture itself, for an
  /// emitter that is its bright parts. The same seed gives the same particles, and `with_progress` and `with_time` do not matter: the swarm is drawn at
  /// any moment of the life.
  pub fn swarm(&self, p_width: u32, p_height: u32, p_photo: Option<&Image>) -> (Swarm, Vec<Option<Image>>) {
    let surface = Surface { source: p_photo, width: p_width, height: p_height };
    let plans = self.plan(&surface, &mut Random::new(self.seed), None);
    let refs: Vec<&Plan> = plans.iter().collect();
    let mut textures = Vec::new();
    let swarm = self.swarm_of(&refs, true, &mut textures);
    (swarm, textures)
  }
}
