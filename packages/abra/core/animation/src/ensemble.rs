use abra_core::{Color, Curve, CurveRange, value_noise};
use rayon::prelude::*;

use crate::schedule::Schedule;
use crate::motion::{MOVER_VALUES, Motion, Mover};

/// How many numbers describe one member in [`Ensemble::new`]: its mover ([`MOVER_VALUES`]: where it starts, the point it turns around, its start
/// speed, and its picks for the velocity in x and y, the orbital turn, the radial push and the noise), then `phase` and `spread` (where it falls in a
/// looping cycle, and how far its age is from the ensemble's progress), the picks for its size, opacity, hue and rotation, and its start color: `1.0`
/// and `red`, `green`, `blue`, `alpha` (0 to 255) when it has one, or five zeros when it has none.
pub const MEMBER_VALUES: usize = MOVER_VALUES + 2 + 4 + 5;

/// How many numbers describe one member at one moment in the lists of a [`Frame`]: `x`, `y`, `size`, `opacity`, `red`, `green`, `blue` (0 to 255),
/// `rotation`, the place it started, `x` and `y`, how much longer than wide it is (`stretch`, 0.0 for not at all) and its `frame` (a whole number).
pub const POSE_VALUES: usize = 12;

/// How many numbers describe one place along the history of a member in the lists of a [`Frame`]: `x`, `y`, `size`, `opacity`, `red`, `green`, `blue`.
pub const HISTORY_POINT_VALUES: usize = 7;

/// How many pieces the color over a life is cut into in the table [`Ensemble::with_lifetime_colors`] takes (it has one more color than this).
pub const LIFETIME_STEPS: usize = 256;

/// How many places along its path the history of a member is worked out at, besides where it is.
const HISTORY_SAMPLES: usize = 24;

/// How a group of members is told apart and played, besides where its members are.
#[derive(Clone, Copy, Debug)]
pub struct EnsembleOptions {
  /// Whether the progress goes round and round: a member's age is the progress plus its own place in the cycle, started over at its start when it
  /// passes the end of its life, so members go on being made. Otherwise a member's age is the progress plus how far it is spread from it, held between
  /// 0 and 1.
  pub looping: bool,
  /// How far back along its path a member's history reaches, as a share of its life. 0 for none.
  pub history: f32,
  /// How much the history fades toward its start: 1 fades to nothing, 0 does not fade.
  pub history_fade: f32,
  /// A whole number the caller gives the ensemble, which comes back on every run it makes, so a caller can tell where each came from.
  pub tag: u32,
}

impl Default for EnsembleOptions {
  fn default() -> Self {
    EnsembleOptions { looping: false, history: 0.0, history_fade: 1.0, tag: 0 }
  }
}

/// How the look of a member follows how fast it is going, from `low` to `high` (pixels over a whole life). Each part is a curve (or a table of colors)
/// read at how far the speed is between the two, from 0.0 at `low` to 1.0 at `high`; a speed outside them is held at the nearest.
#[derive(Clone, Debug)]
pub struct SpeedResponse {
  low: f32,
  high: f32,
  /// A curve whose value (at 0 to 255 along it) is what the size is multiplied by.
  size: Option<Curve>,
  /// A curve whose value is the degrees added to the rotation.
  rotation: Option<Curve>,
  /// The color the member is multiplied by at each of `LIFETIME_STEPS + 1` speeds from `low` to `high`, as whole numbers.
  colors: Option<Vec<[f32; 4]>>,
}

impl SpeedResponse {
  /// A response to speeds from `p_low` to `p_high`, which does nothing until a part is added.
  pub fn new(p_low: f32, p_high: f32) -> Self {
    SpeedResponse { low: p_low.min(p_high), high: p_low.max(p_high), size: None, rotation: None, colors: None }
  }

  /// What the size is multiplied by, as a curve over the speed.
  pub fn with_size(mut self, p_curve: Curve) -> Self {
    self.size = Some(p_curve);
    self
  }

  /// The degrees added to the rotation, as a curve over the speed.
  pub fn with_rotation(mut self, p_curve: Curve) -> Self {
    self.rotation = Some(p_curve);
    self
  }

  /// The color the member is multiplied by over the speed: `LIFETIME_STEPS + 1` colors of four numbers each (`red`, `green`, `blue`, `alpha`, 0 to 255).
  ///
  /// # Panics
  /// When there are not that many numbers.
  pub fn with_colors(mut self, p_colors: &[f32]) -> Self {
    assert_eq!(p_colors.len(), (LIFETIME_STEPS + 1) * 4, "the colors over the speed are {} colors of 4 numbers", LIFETIME_STEPS + 1);
    self.colors = Some(p_colors.chunks_exact(4).map(|color| [color[0], color[1], color[2], color[3]]).collect());
    self
  }

  /// Whether the response changes the color, which a color read from the picture where a thing started cannot show.
  pub fn changes_color(&self) -> bool {
    self.colors.is_some()
  }

  /// How far `p_speed` is from `low` to `high`, from 0.0 to 1.0.
  fn along(&self, p_speed: f32) -> f32 {
    if self.high - self.low < f32::EPSILON { 1.0 } else { ((p_speed - self.low) / (self.high - self.low)).clamp(0.0, 1.0) }
  }

  /// A look changed for a thing going at `p_speed`: `(size, rotation, color, opacity)` in, the same out.
  pub fn apply(&self, p_speed: f32, p_size: f32, p_rotation: f32, p_color: [f32; 3], p_opacity: f32) -> (f32, f32, [f32; 3], f32) {
    let along = self.along(p_speed);
    let (mut size, mut rotation, mut color, mut opacity) = (p_size, p_rotation, p_color, p_opacity);
    if let Some(curve) = &self.size {
      size = (size * curve.value(along * 255.0)).max(0.0);
    }
    if let Some(curve) = &self.rotation {
      rotation += curve.value(along * 255.0);
    }
    if let Some(table) = &self.colors {
      let tint = lifetime_at(table, along);
      color = [0, 1, 2].map(|channel| ((color[channel] * tint[channel]) / 255.0).floor());
      opacity *= tint[3] / 255.0;
    }
    (size, rotation, color, opacity)
  }
}

/// Noise that changes the size and the rotation of things, read where each is: the size changes by up to `size` of itself (0.5 is half again or half as
/// much), the rotation by up to `rotation` degrees, over swirls `swirl` pixels across. The same `seed` gives the same noise.
#[derive(Clone, Copy, Debug)]
pub struct PoseNoise {
  size: f32,
  rotation: f32,
  swirl: f32,
  seed: u64,
}

impl PoseNoise {
  pub fn new(p_size: f32, p_rotation: f32, p_swirl: f32, p_seed: u64) -> Self {
    PoseNoise { size: p_size, rotation: p_rotation, swirl: p_swirl.max(1.0), seed: p_seed }
  }

  /// `(size, rotation)` changed for a thing at `p_position`.
  pub fn apply(&self, p_position: (f32, f32), p_size: f32, p_rotation: f32) -> (f32, f32) {
    let (nx, ny) = (p_position.0 / self.swirl, p_position.1 / self.swirl);
    (
      (p_size * (1.0 + self.size * value_noise(nx, ny, self.seed))).max(0.0),
      p_rotation + self.rotation * value_noise(nx + 31.7, ny + 17.3, self.seed),
    )
  }
}

/// The color over a life (or over a speed) at an age, as whole numbers, mixed between the two it falls between.
fn lifetime_at(p_table: &[[f32; 4]], p_age: f32) -> [f32; 4] {
  let position = p_age.clamp(0.0, 1.0) * LIFETIME_STEPS as f32;
  let at = (position.floor() as usize).min(LIFETIME_STEPS - 1);
  let mix = position - at as f32;
  let (from, to) = (p_table[at], p_table[at + 1]);
  [0, 1, 2, 3].map(|channel| (from[channel] + (to[channel] - from[channel]) * mix).round())
}

/// What each member picked at random for the ranges that shape it, and when it lives.
#[derive(Clone, Copy, Debug)]
struct Member {
  phase: f32,
  spread: f32,
  size_pick: f32,
  opacity_pick: f32,
  hue_pick: f32,
  rotation_pick: f32,
  /// Where its lifetime falls in the range of a [`Schedule`], from 0.0 (shortest) to 1.0 (longest).
  lifetime_pick: f32,
  /// The color it started as, or nothing.
  start: Option<[u8; 4]>,
}

/// The members that start from the members of an ensemble once they are old enough.
#[derive(Clone, Debug)]
struct Child {
  trigger: f32,
  ensemble: Ensemble,
  /// Where each parent's own members start in the child's list, and one more at the end: parent `i` has the members from `first[i]` up to `first[i + 1]`.
  first: Vec<usize>,
}

/// A run of members that came from ensembles with the same tag, as one list of numbers ([`POSE_VALUES`] for each).
#[derive(Clone, Debug, PartialEq)]
pub struct PoseRun {
  pub tag: u32,
  pub values: Vec<f32>,
}

/// A run of histories that came from ensembles with the same tag: the places of all of them one after the other ([`HISTORY_POINT_VALUES`] for each),
/// and how many places each has.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryRun {
  pub tag: u32,
  pub values: Vec<f32>,
  pub lengths: Vec<u32>,
}

/// What an ensemble makes at a moment: its members in the order they come, and the histories of the ones that have moved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
  pub poses: Vec<PoseRun>,
  pub histories: Vec<HistoryRun>,
}

impl Frame {
  /// Puts what `p_next` has after what this has, joining the runs where one ends and the next is the same kind.
  fn append(&mut self, p_next: Frame) {
    for run in p_next.poses {
      match self.poses.last_mut() {
        Some(last) if last.tag == run.tag => last.values.extend(run.values),
        _ => self.poses.push(run),
      }
    }
    for run in p_next.histories {
      match self.histories.last_mut() {
        Some(last) if last.tag == run.tag => {
          last.values.extend(run.values);
          last.lengths.extend(run.lengths);
        }
        _ => self.histories.push(run),
      }
    }
  }
}

/// How many members a run has to have before they are worked out on all the cores; a shorter run is not worth the hand-over.
const PARALLEL_FROM: usize = 256;

/// How many members one core works out before it hands its part back.
const PARALLEL_PIECE: usize = 32;

/// Many things that live and end together: where each starts, what it picked at random, and how its size, opacity, rotation, hue and color follow its
/// life. [`Ensemble::evaluate`] says how every one of them looks at a moment, in one call, as lists of numbers, so thousands of members cost a frame no
/// more than one call does. Nothing here draws or knows what the members are.
#[derive(Clone, Debug)]
pub struct Ensemble {
  motion: Motion,
  movers: Vec<Mover>,
  members: Vec<Member>,
  options: EnsembleOptions,
  size: CurveRange,
  opacity: CurveRange,
  rotation: CurveRange,
  hue: CurveRange,
  /// The color over a life at each of `LIFETIME_STEPS + 1` ages, as whole numbers, or nothing when there is none.
  lifetime: Option<Vec<[f32; 4]>>,
  /// When the members start and how long they last, in seconds. With it, the number given to [`Ensemble::evaluate`] is the time in seconds, and
  /// `phase`, `spread` and the loop of the options are not used.
  schedule: Option<Schedule>,
  /// How the size, rotation and color follow how fast a member is going.
  speed: Option<SpeedResponse>,
  /// Noise read where a member is that changes its size and its rotation.
  pose_noise: Option<PoseNoise>,
  /// How much longer a member is for each pixel over a whole life it goes. A member that is stretched is turned the way it goes.
  velocity_stretch: f32,
  /// How many frames a member goes through over its life, and how many times it plays them.
  frames: (u32, f32),
  children: Vec<Child>,
}

impl Ensemble {
  /// An ensemble of the members in `p_values`, [`MEMBER_VALUES`] numbers for each, that move as `p_motion` says. The size, opacity, rotation and hue
  /// are ranges of the life, read with each member's pick.
  ///
  /// # Panics
  /// When the length of `p_values` is not a whole number of members.
  pub fn new(
    p_motion: Motion, p_values: &[f32], p_options: EnsembleOptions, p_size: CurveRange, p_opacity: CurveRange, p_rotation: CurveRange, p_hue: CurveRange,
  ) -> Ensemble {
    assert!(p_values.len() % MEMBER_VALUES == 0, "a member is {MEMBER_VALUES} numbers");
    let mut movers = Vec::new();
    let mut members = Vec::new();
    for value in p_values.chunks_exact(MEMBER_VALUES) {
      movers.push(Mover::from_values(&value[..MOVER_VALUES]));
      let rest = &value[MOVER_VALUES..];
      let start = (rest[6] != 0.0).then(|| [rest[7], rest[8], rest[9], rest[10]].map(|channel| channel.clamp(0.0, 255.0) as u8));
      members.push(Member { phase: rest[0], spread: rest[1], size_pick: rest[2], opacity_pick: rest[3], hue_pick: rest[4], rotation_pick: rest[5], lifetime_pick: 0.0, start });
    }
    Ensemble {
      motion: p_motion,
      movers,
      members,
      options: p_options,
      size: p_size,
      opacity: p_opacity,
      rotation: p_rotation,
      hue: p_hue,
      lifetime: None,
      schedule: None,
      speed: None,
      pose_noise: None,
      velocity_stretch: 0.0,
      frames: (1, 1.0),
      children: Vec::new(),
    }
  }

  /// Makes the look of the members follow how fast they are going: see [`SpeedResponse`].
  pub fn with_speed_response(mut self, p_response: SpeedResponse) -> Ensemble {
    self.speed = Some(p_response);
    self
  }

  /// Stretches each member along the way it goes, and turns it that way: it is `1 + speed * p_scale` times as long, with the speed in pixels over a
  /// whole life. The rotation of the member is then added to the way it goes.
  pub fn with_velocity_stretch(mut self, p_scale: f32) -> Ensemble {
    self.velocity_stretch = p_scale.max(0.0);
    self
  }

  /// Plays `p_count` frames along each member's life, `p_cycles` times (1.0 plays them once). Each pose says which frame it is.
  pub fn with_frames(mut self, p_count: u32, p_cycles: f32) -> Ensemble {
    self.frames = (p_count.max(1), p_cycles.max(0.0));
    self
  }

  /// Noise that changes the size and the rotation of the members, read where each is: see [`PoseNoise`].
  pub fn with_pose_noise(mut self, p_noise: PoseNoise) -> Ensemble {
    self.pose_noise = Some(p_noise);
    self
  }

  /// Makes the members last over time: member `i` is thing `i` of `p_schedule`, starting when it says and lasting a time picked with
  /// `p_lifetime_picks[i]` (0.0 to 1.0). From then on [`Ensemble::evaluate`] takes the time in seconds, and a member that is not alive then is left
  /// out. The ensembles that start from a member are told its age (0 to 1) in place of the time.
  ///
  /// # Panics
  /// When there are not as many members as things in the schedule, or a pick for each.
  pub fn with_schedule(mut self, p_schedule: Schedule, p_lifetime_picks: &[f32]) -> Ensemble {
    assert_eq!(p_schedule.count(), self.members.len(), "there is a member for each thing the schedule makes");
    assert_eq!(p_lifetime_picks.len(), self.members.len(), "there is a lifetime pick for each member");
    for (member, pick) in self.members.iter_mut().zip(p_lifetime_picks) {
      member.lifetime_pick = *pick;
    }
    self.schedule = Some(p_schedule);
    self
  }

  /// The color the members are multiplied by over a life, as `LIFETIME_STEPS + 1` colors of four numbers each (`red`, `green`, `blue`, `alpha`, 0 to 255)
  /// from the start to the end.
  ///
  /// # Panics
  /// When there are not that many numbers.
  pub fn with_lifetime_colors(mut self, p_colors: &[f32]) -> Ensemble {
    assert_eq!(p_colors.len(), (LIFETIME_STEPS + 1) * 4, "the colors over a life are {} colors of 4 numbers", LIFETIME_STEPS + 1);
    self.lifetime = Some(p_colors.chunks_exact(4).map(|color| [color[0], color[1], color[2], color[3]]).collect());
    self
  }

  /// An ensemble that starts from the members of this one once each is `p_trigger` (0 to 1) of the way through its life. `p_counts` says how many
  /// members of `p_child` belong to each member of this ensemble, in order, and they are the members of `p_child` one after the other.
  ///
  /// # Panics
  /// When there is not a count for each member, or the counts do not add up to the members of the child.
  pub fn with_child(mut self, p_trigger: f32, p_child: Ensemble, p_counts: &[u32]) -> Ensemble {
    assert_eq!(p_counts.len(), self.members.len(), "there is a count for each member");
    let mut first = vec![0];
    for count in p_counts {
      first.push(first.last().unwrap() + *count as usize);
    }
    assert_eq!(*first.last().unwrap(), p_child.members.len(), "the counts add up to the members of the child");
    self.children.push(Child { trigger: p_trigger.clamp(0.0, 1.0), ensemble: p_child, first });
    self
  }

  /// How many members there are in the ensemble, not counting the ones that start from them.
  pub fn len(&self) -> usize {
    self.members.len()
  }

  pub fn is_empty(&self) -> bool {
    self.members.is_empty()
  }

  /// How the ensemble looks `p_progress` (0 to 1) of the way through its life, and the ensembles that start from it as well. The members of a child
  /// come before the member they start from.
  pub fn evaluate(&self, p_progress: f32) -> Frame {
    let mut frame = Frame::default();
    self.collect(0..self.members.len(), p_progress, &mut frame);
    frame
  }

  /// One member at one age, with its opacity scaled by `p_fade`: `(x, y, size, opacity, color, rotation)`.
  fn placed(&self, p_index: usize, p_age: f32, p_fade: f32) -> ([f32; 2], f32, f32, [f32; 3], f32) {
    let member = &self.members[p_index];
    let lifetime = self.lifetime.as_ref().map(|table| lifetime_at(table, p_age));
    let product = |a: f32, b: f32| ((a * b) / 255.0).floor();
    let (mut r, mut g, mut b, a) = match (member.start, lifetime) {
      (Some(start), Some(life)) => (product(start[0] as f32, life[0]), product(start[1] as f32, life[1]), product(start[2] as f32, life[2]), product(start[3] as f32, life[3])),
      (Some(start), None) => (start[0] as f32, start[1] as f32, start[2] as f32, start[3] as f32),
      (None, Some(life)) => (life[0], life[1], life[2], life[3]),
      (None, None) => (255.0, 255.0, 255.0, 255.0),
    };
    if !self.hue.is_zero() {
      let turned = Color::from_rgba(r as u8, g as u8, b as u8, a as u8).with_hue_rotated(self.hue.at(p_age, member.hue_pick)).rgba();
      (r, g, b) = (turned.0 as f32, turned.1 as f32, turned.2 as f32);
    }
    let position = self.motion.at(&self.movers[p_index], p_age);
    let mut size = self.size.at(p_age, member.size_pick).max(0.0);
    let mut opacity = self.opacity.at(p_age, member.opacity_pick).clamp(0.0, 1.0) * (a / 255.0) * p_fade;
    let mut rotation = self.rotation.at(p_age, member.rotation_pick);
    let mut color = [r, g, b];
    if let Some(response) = &self.speed {
      (size, rotation, color, opacity) = response.apply(self.motion.speed_at(&self.movers[p_index], p_age), size, rotation, color, opacity);
    }
    if let Some(noise) = &self.pose_noise {
      (size, rotation) = noise.apply((position.0, position.1), size, rotation);
    }
    ([position.0, position.1], size, opacity, color, rotation)
  }

  /// Adds the members in `p_range` to `p_frame`, each with the ones that start from it going down first.
  ///
  /// A long run of members is worked out on all the cores at once, each member on its own, and put back together in order.
  fn collect(&self, p_range: std::ops::Range<usize>, p_progress: f32, p_frame: &mut Frame) {
    if p_range.len() < PARALLEL_FROM {
      for index in p_range {
        self.collect_one(index, p_progress, p_frame);
      }
      return;
    }
    // In pieces of a few dozen, so each core fills a frame of its own and few have to be put together.
    let starts: Vec<usize> = p_range.clone().step_by(PARALLEL_PIECE).collect();
    let parts: Vec<Frame> = starts
      .into_par_iter()
      .map(|start| {
        let mut part = Frame::default();
        for index in start..(start + PARALLEL_PIECE).min(p_range.end) {
          self.collect_one(index, p_progress, &mut part);
        }
        part
      })
      .collect();
    for part in parts {
      p_frame.append(part);
    }
  }

  /// Adds one member to `p_frame`, with the ones that start from it going down first.
  fn collect_one(&self, index: usize, p_progress: f32, p_frame: &mut Frame) {
    let options = &self.options;
    let member = &self.members[index];
    let age = match &self.schedule {
      Some(schedule) => match schedule.age(index, p_progress, member.lifetime_pick) {
        Some(age) => age,
        None => return,
      },
      None if options.looping => (p_progress + member.phase) % 1.0,
      None => (p_progress + member.spread).clamp(0.0, 1.0),
    };
    // The ensembles that start from a member that lasts over time play as it ages.
    let child_progress = if self.schedule.is_some() { age } else { p_progress };
    for child in &self.children {
      if age >= child.trigger {
        child.ensemble.collect(child.first[index]..child.first[index + 1], child_progress, p_frame);
      }
    }

    let (position, size, opacity, color, mut rotation) = self.placed(index, age, 1.0);
    let mut stretch = 0.0;
    if self.velocity_stretch > 0.0 {
      let (vx, vy) = self.motion.velocity_at(&self.movers[index], age);
      stretch = vx.hypot(vy) * self.velocity_stretch;
      if stretch > 0.0 {
        rotation += vy.atan2(vx).to_degrees();
      }
    }
    let frame = if self.frames.0 > 1 { ((age * self.frames.1).fract() * self.frames.0 as f32) as u32 } else { 0 };

    // The history is the member at earlier ages, along its path. One that has not moved far enough to be told from a point is left out.
    let span = options.history.min(age);
    if span > 0.0 {
      let samples: Vec<_> = (0..=HISTORY_SAMPLES)
        .map(|i| {
          let behind = 1.0 - i as f32 / HISTORY_SAMPLES as f32;
          self.placed(index, age - span * behind, 1.0 - options.history_fade * behind)
        })
        .collect();
      let length: f32 = samples.windows(2).map(|pair| (pair[1].0[0] - pair[0].0[0]).hypot(pair[1].0[1] - pair[0].0[1])).sum();
      if length >= 0.5 {
        if p_frame.histories.last().is_none_or(|run| run.tag != options.tag) {
          p_frame.histories.push(HistoryRun { tag: options.tag, values: Vec::new(), lengths: Vec::new() });
        }
        let run = p_frame.histories.last_mut().unwrap();
        for (sample_position, sample_size, sample_opacity, sample_color, _) in &samples {
          run.values.extend([sample_position[0], sample_position[1], *sample_size, *sample_opacity, sample_color[0], sample_color[1], sample_color[2]]);
        }
        run.lengths.push(samples.len() as u32);
      }
    }

    if p_frame.poses.last().is_none_or(|run| run.tag != options.tag) {
      p_frame.poses.push(PoseRun { tag: options.tag, values: Vec::new() });
    }
    let started = self.movers[index].born;
    p_frame.poses.last_mut().unwrap().values.extend([
      position[0],
      position[1],
      size,
      opacity,
      color[0],
      color[1],
      color[2],
      rotation,
      started.0,
      started.1,
      stretch,
      frame as f32,
    ]);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A member's numbers: it starts at `x`, 10, has no motion of its own, and has no start color.
  fn member(p_x: f32) -> Vec<f32> {
    let mut values = vec![p_x, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    values.extend([0.0, 0.0, 0.5, 0.5, 0.5, 0.5]);
    values.extend([0.0, 0.0, 0.0, 0.0, 0.0]);
    values
  }

  fn ensemble(p_values: &[f32], p_options: EnsembleOptions) -> Ensemble {
    Ensemble::new(Motion::new(), p_values, p_options, 4.0.into(), 0.5.into(), 30.0.into(), 0.0.into())
  }

  #[test]
  fn a_member_is_a_pose_of_the_numbers_of_its_life() {
    let frame = ensemble(&member(7.0), EnsembleOptions::default()).evaluate(0.5);
    assert_eq!(frame.poses, vec![PoseRun { tag: 0, values: vec![7.0, 10.0, 4.0, 0.5, 255.0, 255.0, 255.0, 30.0, 7.0, 10.0, 0.0, 0.0] }]);
    assert!(frame.histories.is_empty());
  }

  #[test]
  fn the_color_is_the_start_color_times_the_color_over_the_life() {
    let mut values = member(0.0);
    let len = values.len();
    values[len - 5..].copy_from_slice(&[1.0, 200.0, 100.0, 50.0, 255.0]);
    let half: Vec<f32> = (0..=LIFETIME_STEPS).flat_map(|_| [127.0, 255.0, 0.0, 255.0]).collect();
    let frame = ensemble(&values, EnsembleOptions::default()).with_lifetime_colors(&half).evaluate(0.5);
    let pose = &frame.poses[0].values;
    assert_eq!(&pose[4..7], &[(200.0 * 127.0 / 255.0f32).floor(), 100.0, 0.0]);
    assert_eq!(pose[3], 0.5);
  }

  #[test]
  fn with_a_schedule_a_member_is_made_only_while_it_is_alive() {
    let mut values = member(1.0);
    values.extend(member(2.0));
    let schedule = Schedule::new(5.0).with_burst(0.0, 1).with_burst(2.0, 1).with_lifetime(1.0, 1.0);
    let ensemble = ensemble(&values, EnsembleOptions::default()).with_schedule(schedule, &[0.0, 0.0]);
    let xs = |time: f32| ensemble.evaluate(time).poses.first().map(|run| run.values.iter().step_by(POSE_VALUES).copied().collect::<Vec<_>>());
    assert_eq!(xs(0.5), Some(vec![1.0]));
    assert_eq!(xs(1.5), None);
    assert_eq!(xs(2.5), Some(vec![2.0]));
  }

  #[test]
  #[should_panic(expected = "a member for each thing")]
  fn a_schedule_that_makes_a_different_number_of_things_panics() {
    ensemble(&member(0.0), EnsembleOptions::default()).with_schedule(Schedule::new(1.0).with_burst(0.0, 2), &[0.0]);
  }

  #[test]
  fn size_and_rotation_follow_the_speed() {
    let mut values = member(0.0);
    values[4] = 100.0;
    let response = SpeedResponse::new(0.0, 200.0)
      .with_size(Curve::new([(0.0, 1.0), (255.0, 3.0)]))
      .with_rotation(Curve::new([(0.0, 0.0), (255.0, 90.0)]));
    let moving = ensemble(&values, EnsembleOptions::default()).with_speed_response(response);
    let pose = &moving.evaluate(0.5).poses[0].values;
    assert!((pose[2] - 4.0 * 2.0).abs() < 0.05, "half speed, size 4 times 2: {}", pose[2]);
    assert!((pose[7] - (30.0 + 45.0)).abs() < 0.5, "{}", pose[7]);
  }

  #[test]
  fn noise_changes_size_and_rotation_and_none_leaves_them() {
    let plain = ensemble(&member(7.0), EnsembleOptions::default()).evaluate(0.5);
    let noisy = ensemble(&member(7.0), EnsembleOptions::default()).with_pose_noise(PoseNoise::new(0.0, 0.0, 20.0, 3)).evaluate(0.5);
    assert_eq!(plain, noisy);
    let wild = ensemble(&member(7.0), EnsembleOptions::default()).with_pose_noise(PoseNoise::new(0.5, 40.0, 20.0, 3)).evaluate(0.5);
    assert_ne!(plain.poses[0].values[2], wild.poses[0].values[2]);
    assert_ne!(plain.poses[0].values[7], wild.poses[0].values[7]);
  }

  #[test]
  fn velocity_stretch_turns_a_member_the_way_it_goes_and_lengthens_it() {
    let mut values = member(0.0);
    values[4] = 30.0;
    values[5] = 40.0;
    let pose = &ensemble(&values, EnsembleOptions::default()).with_velocity_stretch(0.02).evaluate(0.5).poses[0].values;
    assert!((pose[10] - 1.0).abs() < 0.01, "50 pixels a life at 0.02: {}", pose[10]);
    assert!((pose[7] - (30.0 + 53.13)).abs() < 0.1, "{}", pose[7]);
  }

  #[test]
  fn frames_play_along_the_life() {
    let frames = |age: f32| ensemble(&member(0.0), EnsembleOptions::default()).with_frames(4, 1.0).evaluate(age).poses[0].values[11];
    assert_eq!([0.1, 0.3, 0.6, 0.9].map(frames), [0.0, 1.0, 2.0, 3.0]);
  }

  #[test]
  fn a_looping_member_starts_over_and_one_that_is_not_stops() {
    let mut values = member(0.0);
    values[MOVER_VALUES] = 0.75;
    values[MOVER_VALUES + 1] = 0.75;
    let moving = Motion::new().with_velocity(40.0, 0.0);
    let make = |options| Ensemble::new(moving.clone(), &values, options, 4.0.into(), 0.5.into(), 0.0.into(), 0.0.into());
    // In a loop the age is 0.25 when the progress is 0.5, and held at the end of the life (1.0) when it is not.
    let looping = make(EnsembleOptions { looping: true, ..EnsembleOptions::default() }).evaluate(0.5).poses[0].values[0];
    let held = make(EnsembleOptions::default()).evaluate(0.5).poses[0].values[0];
    assert!((looping - 10.0).abs() < 1e-3, "{looping}");
    assert!((held - 40.0).abs() < 1e-3, "{held}");
  }

  #[test]
  fn a_history_is_left_when_a_member_has_moved_and_not_when_it_has_not() {
    let options = EnsembleOptions { history: 0.5, ..EnsembleOptions::default() };
    let still = ensemble(&member(0.0), options).evaluate(0.5);
    assert!(still.histories.is_empty(), "a member that does not move leaves no history");
    let moving = Motion::new().with_velocity(40.0, 0.0);
    let frame = Ensemble::new(moving, &member(0.0), options, 4.0.into(), 0.5.into(), 0.0.into(), 0.0.into()).evaluate(0.5);
    assert_eq!(frame.histories.len(), 1);
    assert_eq!(frame.histories[0].lengths, vec![25]);
    assert_eq!(frame.histories[0].values.len(), 25 * HISTORY_POINT_VALUES);
  }

  #[test]
  fn children_come_before_their_parent_once_the_parent_is_old_enough() {
    let parent = ensemble(&member(1.0), EnsembleOptions::default());
    let child = ensemble(&[member(2.0), member(3.0)].concat(), EnsembleOptions { tag: 1, ..EnsembleOptions::default() });
    let both = parent.with_child(0.5, child, &[2]);
    let after = both.evaluate(0.75);
    let order: Vec<(u32, f32)> = after.poses.iter().map(|run| (run.tag, run.values[0])).collect();
    assert_eq!(order, vec![(1, 2.0), (0, 1.0)]);
    assert_eq!(after.poses[0].values.len(), 2 * POSE_VALUES);
    // Before the trigger only the parent is there.
    assert_eq!(both.evaluate(0.25).poses.len(), 1);
  }

  #[test]
  fn the_same_frame_comes_from_one_core_and_from_many() {
    let values: Vec<f32> = (0..1000).flat_map(|i| member(i as f32)).collect();
    let moving = Motion::new().with_velocity(10.0, 5.0);
    let options = EnsembleOptions { history: 0.3, ..EnsembleOptions::default() };
    let many = Ensemble::new(moving, &values, options, 4.0.into(), 0.5.into(), 0.0.into(), 0.0.into()).evaluate(0.6);
    assert_eq!(many.poses[0].values.len(), 1000 * POSE_VALUES);
    // The members come in the order they were given.
    let xs: Vec<f32> = many.poses[0].values.chunks(POSE_VALUES).map(|pose| pose[8]).collect();
    assert!(xs.windows(2).all(|pair| pair[0] < pair[1]));
  }

  #[test]
  #[should_panic(expected = "a member is 22 numbers")]
  fn values_that_are_not_whole_members_panic() {
    ensemble(&[1.0, 2.0], EnsembleOptions::default());
  }

  #[test]
  #[should_panic(expected = "the counts add up")]
  fn counts_that_do_not_match_the_children_panic() {
    let parent = ensemble(&member(1.0), EnsembleOptions::default());
    parent.with_child(0.5, ensemble(&member(2.0), EnsembleOptions::default()), &[2]);
  }
}
