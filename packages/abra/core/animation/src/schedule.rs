/// When things are born and how long each lives, over time in seconds: a steady `rate` of things a second, `bursts` of many at once, a `delay`
/// before the first, and a lifetime picked from a range. The system can run once for its `duration` or go round and round.
///
/// Things are numbered: the bursts first, in the order they were added, then the steady stream. [`Schedule::count`] says how many there are for one run,
/// and [`Schedule::age`] says how old thing `i` is at a moment, or that it is not alive then.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
  duration: f32,
  looping: bool,
  rate: f32,
  bursts: Vec<(f32, u32)>,
  delay: f32,
  lifetime: (f32, f32),
}

impl Schedule {
  /// A system that runs for `p_duration` seconds, with nothing born yet and things that live one second.
  pub fn new(p_duration: f32) -> Self {
    Schedule { duration: p_duration.max(f32::EPSILON), looping: false, rate: 0.0, bursts: Vec::new(), delay: 0.0, lifetime: (1.0, 1.0) }
  }

  /// Whether the system starts over when its duration is up, so things go on being born.
  pub fn with_looping(mut self, p_looping: bool) -> Self {
    self.looping = p_looping;
    self
  }

  /// How many things are born each second, evenly spread over the duration.
  pub fn with_rate(mut self, p_rate: f32) -> Self {
    self.rate = p_rate.max(0.0);
    self
  }

  /// `p_count` things born together `p_time` seconds in. Each call adds another burst.
  pub fn with_burst(mut self, p_time: f32, p_count: u32) -> Self {
    self.bursts.push((p_time.max(0.0), p_count));
    self
  }

  /// Seconds before anything is born.
  pub fn with_delay(mut self, p_delay: f32) -> Self {
    self.delay = p_delay.max(0.0);
    self
  }

  /// How long each thing lives, picked between `p_min` and `p_max` seconds.
  pub fn with_lifetime(mut self, p_min: f32, p_max: f32) -> Self {
    let (low, high) = (p_min.min(p_max).max(f32::EPSILON), p_min.max(p_max).max(f32::EPSILON));
    self.lifetime = (low, high);
    self
  }

  /// How many things there are in one run: the bursts, and the rate over the duration, rounded.
  pub fn count(&self) -> usize {
    self.bursts.iter().map(|(_, count)| *count as usize).sum::<usize>() + self.streamed()
  }

  fn streamed(&self) -> usize {
    (self.rate * self.duration).round() as usize
  }

  /// Seconds into a run that thing `p_index` is born, or nothing when there is no such thing.
  pub fn birth(&self, p_index: usize) -> Option<f32> {
    let mut left = p_index;
    for (time, count) in &self.bursts {
      if left < *count as usize {
        return Some(self.delay + time);
      }
      left -= *count as usize;
    }
    (left < self.streamed()).then(|| self.delay + left as f32 / self.rate)
  }

  /// How long a thing lives, in seconds, for its pick `p_pick` (0.0 for the shortest life, 1.0 for the longest).
  pub fn lifetime(&self, p_pick: f32) -> f32 {
    self.lifetime.0 + (self.lifetime.1 - self.lifetime.0) * p_pick
  }

  /// How far through its life thing `p_index` is at `p_time` seconds, from 0.0 (born) to 1.0 (gone), or nothing when it is not alive then: not born
  /// yet, or past the end of its life. In a loop the time starts over each time the duration is up, so a thing is alive again each time round.
  /// To start a loop already full of things (a prewarm), ask for a time a duration later.
  pub fn age(&self, p_index: usize, p_time: f32, p_pick: f32) -> Option<f32> {
    let birth = self.birth(p_index)?;
    let lifetime = self.lifetime(p_pick);
    let mut since = p_time - birth;
    if self.looping {
      since = since.rem_euclid(self.delay + self.duration);
    }
    (0.0..=lifetime).contains(&since).then(|| since / lifetime)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_count_is_the_bursts_and_the_rate_over_the_duration() {
    assert_eq!(Schedule::new(2.0).with_rate(10.0).with_burst(0.0, 5).with_burst(1.0, 3).count(), 28);
    assert_eq!(Schedule::new(2.0).count(), 0);
  }

  #[test]
  fn bursts_come_first_and_the_stream_is_evenly_spaced_after_the_delay() {
    let schedule = Schedule::new(2.0).with_rate(2.0).with_burst(0.5, 2).with_delay(1.0);
    assert_eq!([0, 1].map(|i| schedule.birth(i)), [Some(1.5), Some(1.5)]);
    assert_eq!([2, 3, 4, 5].map(|i| schedule.birth(i)), [Some(1.0), Some(1.5), Some(2.0), Some(2.5)]);
    assert_eq!(schedule.birth(6), None);
  }

  #[test]
  fn a_thing_is_not_alive_before_it_is_born_or_after_it_dies() {
    let schedule = Schedule::new(5.0).with_burst(1.0, 1).with_lifetime(2.0, 2.0);
    assert_eq!(schedule.age(0, 0.5, 0.0), None);
    assert_eq!(schedule.age(0, 1.0, 0.0), Some(0.0));
    assert_eq!(schedule.age(0, 2.0, 0.0), Some(0.5));
    assert_eq!(schedule.age(0, 3.5, 0.0), None);
  }

  #[test]
  fn the_pick_chooses_the_lifetime() {
    let schedule = Schedule::new(5.0).with_burst(0.0, 1).with_lifetime(1.0, 3.0);
    assert_eq!(schedule.age(0, 1.0, 0.0), Some(1.0));
    assert_eq!(schedule.age(0, 1.0, 1.0), Some(1.0 / 3.0));
  }

  #[test]
  fn a_loop_brings_things_back_each_time_round() {
    let schedule = Schedule::new(4.0).with_burst(0.0, 1).with_lifetime(1.0, 1.0).with_looping(true);
    assert_eq!(schedule.age(0, 4.5, 0.0), Some(0.5));
    assert_eq!(schedule.age(0, 6.0, 0.0), None);
    assert_eq!(Schedule::new(4.0).with_burst(0.0, 1).age(0, 4.5, 0.0), None);
  }
}
