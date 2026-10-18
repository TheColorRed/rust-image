//! Sequences: tweens played in order, with the tweens inside each step running together.
//!
//! ```ignore
//! let playback = component.play([
//!   Step::together([Animation::new(&a, 1.0), Animation::new(&b, 0.5), Animation::new(&c, 0.0)]),
//!   Step::together([Animation::new(&e, 10.0)]).with_delay(Duration::from_secs(2)),
//! ]);
//! playback.pause(); // holds everything where it is
//! playback.play(); // carries on from there
//! playback.restart(); // puts the properties back where they started and plays again
//! ```
//!
//! The animations of a step start at the same moment and run in parallel. The next step starts once all of them have arrived, and
//! after its own delay if it has one (`with_delay`): the two seconds above are the wait between `a`, `b` and `c` arriving and `e`
//! starting. Every animation reads where its property is when its step begins, so a step moves from wherever the one before it left
//! things.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pub_sub::{BehaviorSubject, Observer};

use crate::Component;
use crate::component::{RunningTween, WeakComponent};
use crate::tween::{Animatable, TweenOptions};

/// What a tween that belongs to a sequence is linked to: the count of tweens its step is waiting for, and the sequence's pause.
type Link = (Arc<AtomicUsize>, BehaviorSubject<bool>);

/// One property moving to a target, with the type of the property hidden so a step can hold properties of any kind.
#[derive(Clone)]
pub struct Animation {
  property: usize,
  options: TweenOptions,
  /// Starts the tween on a component. It reads the property then, not when the animation was written down.
  start: Arc<dyn Fn(&Component, TweenOptions, Link) + Send + Sync>,
  /// Reads where the property is now, and gives back what puts it there again.
  origin: Arc<dyn Fn() -> Box<dyn Fn() + Send + Sync> + Send + Sync>,
}

impl Animation {
  /// `p_property` moving to `p_target`, with the default options.
  pub fn new<T: Animatable>(p_property: &BehaviorSubject<T>, p_target: T) -> Self {
    let (moving, remembered) = (p_property.clone(), p_property.clone());
    Self {
      property: p_property.id(),
      options: TweenOptions::default(),
      start: Arc::new(move |component, options, link| {
        component.start_tween(&moving, p_target.clone(), options, Some(link))
      }),
      origin: Arc::new(move || {
        let (property, value) = (remembered.clone(), remembered.peek());
        Box::new(move || property.next(value.clone()))
      }),
    }
  }

  /// This animation with its own duration and easing.
  pub fn with_options(self, p_options: TweenOptions) -> Self {
    Self {
      options: p_options,
      ..self
    }
  }
}

/// Animations that run together, and how long to wait before they start.
#[derive(Clone)]
pub struct Step {
  /// The animations that run together in this step.
  animations: Vec<Animation>,
  /// How long to wait before starting the animations in this step.
  delay: Duration,
}

impl Step {
  /// A step that runs `p_animations` all at once.
  pub fn together(p_animations: impl IntoIterator<Item = Animation>) -> Self {
    Self {
      animations: p_animations.into_iter().collect(),
      delay: Duration::ZERO,
    }
  }

  /// A step that only waits for `p_duration`, for a wait at the end of a sequence (so `finished` comes later) or between two steps.
  pub fn wait(p_duration: Duration) -> Self {
    Self::together([]).with_delay(p_duration)
  }

  /// This step, waiting `p_delay` before its animations start. The wait begins when the animations of the step before it have
  /// arrived, and the animations read their properties when it ends.
  pub fn with_delay(self, p_delay: Duration) -> Self {
    Self { delay: p_delay, ..self }
  }
}

/// How a sequence plays. The default plays it once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayOptions {
  /// Whether the sequence starts again when it reaches the end, putting its properties back as [`Playback::restart`] does. It
  /// then never finishes by itself; [`Playback::stop`] ends it.
  pub looping: bool,
}

impl PlayOptions {
  /// These options with looping on or off.
  pub fn with_looping(self, p_looping: bool) -> Self {
    Self { looping: p_looping, ..self }
  }
}

/// What a sequence and the handle to it share.
struct Shared {
  component: WeakComponent,
  /// The steps as they were written, for playing again.
  steps: Vec<Step>,
  /// Whether it starts again at the end. A stream, so it can be changed while the sequence plays.
  looping: BehaviorSubject<bool>,
  /// Each property the sequence has touched, and what puts it back where it was before the sequence first moved it.
  origins: Mutex<Vec<(usize, Box<dyn Fn() + Send + Sync>)>>,
  paused: BehaviorSubject<bool>,
  finished: BehaviorSubject<bool>,
  stopped: AtomicBool,
  /// Asks a sequence that is still running to start again from the first step.
  restart: AtomicBool,
}

/// A sequence that was started with [`Component::play`].
#[derive(Clone)]
pub struct Playback {
  shared: Arc<Shared>,
}

impl Playback {
  /// Whether the sequence is over, as a stream: `playback.finished().subscribe(..)` hears `true` when the last step has ended or
  /// the sequence was stopped.
  pub fn finished(&self) -> &BehaviorSubject<bool> {
    &self.shared.finished
  }

  /// Whether the sequence is paused, as a stream.
  pub fn paused(&self) -> &BehaviorSubject<bool> {
    &self.shared.paused
  }

  /// Whether the sequence starts again when it reaches the end, as a stream: `playback.looping().next(true)` turns it on while
  /// it plays, and a sequence that is already over stays over.
  pub fn looping(&self) -> &BehaviorSubject<bool> {
    &self.shared.looping
  }

  /// Holds the sequence where it is: the animations stop moving, delays stop counting, and the engine is not asked for frames. It
  /// carries on from the same place after [`play`](Self::play).
  pub fn pause(&self) {
    self.shared.paused.next(true);
  }

  /// Carries on a paused sequence from where it was held.
  pub fn play(&self) {
    if self.shared.paused.peek() {
      self.shared.paused.next(false);
      if let Some(component) = self.shared.component.upgrade() {
        component.wake_animations();
      }
    }
  }

  /// Puts every property the sequence has moved back where it was before the sequence first touched it, and plays it again from
  /// the first step. It plays whether the sequence was running, paused, stopped or finished.
  pub fn restart(&self) {
    let shared = &self.shared;
    let Some(component) = shared.component.upgrade() else { return };
    shared.stopped.store(false, Ordering::SeqCst);
    shared.paused.next(false);
    if shared.finished.peek() {
      // Nothing is running, so it can be put back and started now.
      shared.origins.lock().unwrap().drain(..).for_each(|(_, restore)| restore());
      shared.finished.next(false);
      Sequence::launch(shared, &component);
    } else {
      // The sequence puts itself back on its next frame, which comes before the tweens it started.
      shared.restart.store(true, Ordering::SeqCst);
      component.wake_animations();
    }
  }

  /// Stops the sequence: the animations of the step that is running stop where they are, and no later step starts.
  pub fn stop(&self) {
    self.shared.stopped.store(true, Ordering::SeqCst);
  }
}

enum Phase {
  /// The next step has not started.
  Ready,
  /// A step was picked and its delay is running down; its animations start when it ends.
  Delaying { left: Duration, step: Step },
  /// A step's animations are running; this counts down to the last one arriving.
  Playing { pending: Arc<AtomicUsize> },
}

struct Sequence {
  /// Names this sequence among the component's tweens. Odd, so it can never be the address of a property.
  id: usize,
  shared: Arc<Shared>,
  /// The steps that have not started.
  steps: VecDeque<Step>,
  phase: Phase,
  /// The properties of the step that is running, to stop them if the sequence is stopped.
  running: Vec<usize>,
}

/// Gives each sequence its own odd name.
static NEXT_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

impl Sequence {
  /// Starts playing the steps of `p_shared` from the first on `p_component`.
  fn launch(p_shared: &Arc<Shared>, p_component: &Component) {
    p_component.start_running(Box::new(Sequence {
      id: NEXT_SEQUENCE.fetch_add(1, Ordering::SeqCst) * 2 + 1,
      shared: Arc::clone(p_shared),
      steps: p_shared.steps.iter().cloned().collect(),
      phase: Phase::Ready,
      running: Vec::new(),
    }));
  }

  /// Starts the animations of `p_step`. Returns whether the step has nothing to animate, so the caller can go straight on to the
  /// next step; otherwise the tweens need their frames.
  fn begin(&mut self, p_component: &Component, p_step: Step) -> bool {
    let pending = Arc::new(AtomicUsize::new(p_step.animations.len()));
    self.running = p_step.animations.iter().map(|animation| animation.property).collect();
    {
      let mut origins = self.shared.origins.lock().unwrap();
      for animation in &p_step.animations {
        if !origins.iter().any(|(property, _)| *property == animation.property) {
          origins.push((animation.property, (animation.origin)()));
        }
      }
    }
    for animation in &p_step.animations {
      (animation.start)(p_component, animation.options, (Arc::clone(&pending), self.shared.paused.clone()));
    }
    self.phase = Phase::Playing { pending };
    self.running.is_empty()
  }

  /// Stops what is running, puts the properties back and goes back to the first step.
  fn rewind(&mut self, p_component: &Component) {
    self.running.drain(..).for_each(|property| p_component.stop_property(property));
    self.shared.origins.lock().unwrap().drain(..).for_each(|(_, restore)| restore());
    self.steps = self.shared.steps.iter().cloned().collect();
    self.phase = Phase::Ready;
  }
}

impl RunningTween for Sequence {
  fn property(&self) -> usize {
    self.id
  }

  fn paused(&self) -> bool {
    self.shared.paused.peek()
  }

  fn step(&mut self, p_delta: Duration) -> bool {
    let shared = Arc::clone(&self.shared);
    let Some(component) = shared.component.upgrade() else {
      shared.finished.next(true);
      return true;
    };
    if shared.restart.swap(false, Ordering::SeqCst) {
      shared.stopped.store(false, Ordering::SeqCst);
      self.rewind(&component);
    }
    if shared.stopped.load(Ordering::SeqCst) {
      self.running.drain(..).for_each(|property| component.stop_property(property));
      shared.finished.next(true);
      return true;
    }
    if shared.paused.peek() {
      return false;
    }
    loop {
      match std::mem::replace(&mut self.phase, Phase::Ready) {
        Phase::Ready => {
          let Some(step) = self.steps.pop_front() else {
            if shared.looping.peek() && !shared.steps.is_empty() {
              // Back to the start. The first step begins on the next frame, so steps that take no time cannot spin here.
              self.rewind(&component);
              return false;
            }
            shared.finished.next(true);
            return true;
          };
          if !step.delay.is_zero() {
            // This frame's time does not count toward the delay: the first frame of a sequence covers the time the engine slept.
            self.phase = Phase::Delaying { left: step.delay, step };
            return false;
          }
          if self.begin(&component, step) {
            continue;
          }
          return false;
        }
        Phase::Delaying { left, step } => {
          let left = left.saturating_sub(p_delta);
          if !left.is_zero() {
            self.phase = Phase::Delaying { left, step };
            return false;
          }
          if self.begin(&component, step) {
            continue;
          }
          return false;
        }
        Phase::Playing { pending } => {
          if pending.load(Ordering::SeqCst) > 0 {
            self.phase = Phase::Playing { pending };
            return false;
          }
          // They have all arrived: the next time round the loop picks the next step.
          self.running.clear();
        }
      }
    }
  }
}

impl Component {
  /// Plays `p_steps` in order: the animations inside a step run in parallel, and the next step starts when they have all arrived
  /// and the next step's delay is over. The component keeps the engine ticking until the last step ends. Several sequences can play
  /// at once; if two of them (or a sequence and a plain [`animate`](Self::animate)) move the same property, the newest wins. The
  /// [`Playback`] it returns pauses, plays, restarts and stops the sequence.
  pub fn play(&self, p_steps: impl IntoIterator<Item = Step>) -> Playback {
    self.play_with(p_steps, PlayOptions::default())
  }

  /// [`play`](Self::play) with `p_options`, such as `PlayOptions::default().with_looping(true)`.
  pub fn play_with(&self, p_steps: impl IntoIterator<Item = Step>, p_options: PlayOptions) -> Playback {
    let shared = Arc::new(Shared {
      component: self.downgrade(),
      steps: p_steps.into_iter().collect(),
      looping: BehaviorSubject::new(p_options.looping),
      origins: Mutex::new(Vec::new()),
      paused: BehaviorSubject::new(false),
      finished: BehaviorSubject::new(false),
      stopped: AtomicBool::new(false),
      restart: AtomicBool::new(false),
    });
    Sequence::launch(&shared, self);
    Playback { shared }
  }
}

#[cfg(test)]
mod tests {
  use vessel_engine::{MediaSource, Pacing};

  use super::*;
  use crate::Easing;

  fn linear(p_millis: u64) -> TweenOptions {
    TweenOptions::default().with_duration(Duration::from_millis(p_millis)).with_easing(Easing::Linear)
  }

  #[test]
  fn a_steps_animations_run_together_and_the_next_step_waits_for_them_and_its_delay() {
    let component = Component::new("sequence").with_size(2, 2);
    let (a, b, c) =
      (BehaviorSubject::new(0.0f32), BehaviorSubject::new([0u8, 0, 0, 255]), BehaviorSubject::new(0.0f32));
    let playback = component.play([
      Step::together([
        Animation::new(&a, 1.0).with_options(linear(100)),
        Animation::new(&b, [100, 0, 0, 255]).with_options(linear(100)),
      ]),
      Step::together([Animation::new(&c, 1.0).with_options(linear(100))]).with_delay(Duration::from_millis(50)),
    ]);
    assert_eq!(component.clone().pacing(), Pacing::Continuous);
    let frame = |millis: u64| component.clone().render(Duration::from_millis(millis));

    frame(3000); // the engine had been asleep: the first frame starts step one and counts for nothing
    frame(16); // the tweens' own first frame
    frame(50);
    assert_eq!((a.peek(), b.peek()), (0.5, [50, 0, 0, 255]), "both animations of the step move together");
    assert_eq!(c.peek(), 0.0, "the next step has not started");
    frame(60); // arrives
    assert_eq!((a.peek(), b.peek()), (1.0, [100, 0, 0, 255]));
    frame(10); // the sequence notices, and step two's delay begins
    frame(30);
    assert_eq!(c.peek(), 0.0, "still waiting: 30 of 50 ms");
    frame(20); // the delay ends and step two starts
    frame(16);
    frame(50);
    assert_eq!(c.peek(), 0.5, "step two runs after step one");
    assert!(!playback.finished().peek());
    frame(60);
    frame(1);
    assert_eq!(c.peek(), 1.0);
    assert!(playback.finished().peek());
    assert_eq!(component.clone().pacing(), Pacing::OnDemand, "nothing is left to move, so the engine sleeps");
  }

  #[test]
  fn a_delayed_step_waits_before_it_starts_and_reads_its_property_then() {
    let component = Component::new("sequence").with_size(2, 2);
    let (a, b) = (BehaviorSubject::new(0.0f32), BehaviorSubject::new(0.0f32));
    let playback = component.play([
      Step::together([Animation::new(&a, 10.0).with_options(linear(100))]).with_delay(Duration::from_millis(80)),
      Step::together([Animation::new(&b, 1.0).with_options(linear(100))]).with_delay(Duration::from_millis(40)),
    ]);
    let frame = |millis: u64| component.clone().render(Duration::from_millis(millis));

    frame(3000); // the engine had been asleep: that time is not part of the delay
    frame(50);
    frame(20);
    assert_eq!(a.peek(), 0.0, "70 of the 80 ms delay have passed");
    a.next(2.0); // moved by someone else while the step waits: it starts from here
    frame(20); // the delay ends and the step starts
    frame(16);
    frame(50);
    assert_eq!(a.peek(), 6.0, "from 2 to 10, halfway after 50 of 100 ms");
    frame(60);
    frame(10); // arrived; the sequence notices and picks the next step, whose delay begins
    assert_eq!(a.peek(), 10.0);
    frame(30);
    assert_eq!(b.peek(), 0.0, "the second step is still in its 40 ms delay");
    frame(20); // delay over, step two starts
    frame(16);
    frame(60);
    frame(60); // arrived
    frame(1); // the sequence notices that there is no step left
    assert_eq!(b.peek(), 1.0);
    assert!(playback.finished().peek());
    assert_eq!(component.clone().pacing(), Pacing::OnDemand);
  }

  #[test]
  fn a_wait_step_only_waits_and_stop_ends_the_sequence_where_it_is() {
    let component = Component::new("sequence").with_size(2, 2);
    let (a, b) = (BehaviorSubject::new(0.0f32), BehaviorSubject::new(0.0f32));
    let playback = component.play([
      Step::wait(Duration::from_millis(100)),
      Step::together([Animation::new(&a, 10.0).with_options(linear(100))]),
      Step::together([Animation::new(&b, 1.0)]),
    ]);
    let frame = |millis: u64| component.clone().render(Duration::from_millis(millis));
    frame(1);
    frame(60);
    assert_eq!(a.peek(), 0.0, "the first step is only a wait");
    frame(60);
    frame(16);
    frame(50);
    assert_eq!(a.peek(), 5.0);

    playback.stop();
    frame(16);
    assert!(playback.finished().peek());
    frame(100);
    assert_eq!(
      (a.peek(), b.peek()),
      (5.0, 0.0),
      "the running animation stopped where it was and the last step never began"
    );
    assert_eq!(component.clone().pacing(), Pacing::OnDemand);
  }

  #[test]
  fn pause_holds_everything_where_it_is_and_play_carries_on_from_there() {
    let component = Component::new("sequence").with_size(2, 2);
    let (a, b) = (BehaviorSubject::new(0.0f32), BehaviorSubject::new(0.0f32));
    let playback = component.play([
      Step::together([Animation::new(&a, 10.0).with_options(linear(100))]),
      Step::together([Animation::new(&b, 1.0).with_options(linear(100))]).with_delay(Duration::from_millis(40)),
    ]);
    let frame = |millis: u64| component.clone().render(Duration::from_millis(millis));
    frame(3000);
    frame(16);
    frame(50);
    assert_eq!(a.peek(), 5.0);

    playback.pause();
    assert!(playback.paused().peek());
    assert_eq!(component.clone().pacing(), Pacing::OnDemand, "a paused sequence does not ask for frames");
    frame(500);
    assert_eq!(a.peek(), 5.0, "held where it was, however long it is paused");

    playback.play();
    assert_eq!(component.clone().pacing(), Pacing::Continuous);
    frame(30);
    assert!((a.peek() - 8.0).abs() < 1e-4, "it carries on from 5, not from where the clock is: {}", a.peek());
    frame(30); // arrives
    frame(10); // the sequence notices; the 40 ms delay of step two begins
    playback.pause();
    frame(1000);
    assert_eq!(b.peek(), 0.0, "a delay does not run down while paused");
    playback.play();
    frame(30);
    assert_eq!(b.peek(), 0.0, "30 of 40 ms");
    frame(10); // the delay ends and step two starts
    frame(16);
    frame(100);
    frame(1);
    assert_eq!((b.peek(), playback.finished().peek()), (1.0, true));
  }

  #[test]
  fn a_looping_sequence_restarts_at_the_end_until_it_is_stopped() {
    let component = Component::new("sequence").with_size(2, 2);
    let (a, b) = (BehaviorSubject::new(0.0f32), BehaviorSubject::new(0.0f32));
    let steps = || {
      [
        Step::together([Animation::new(&a, 10.0).with_options(linear(100))]),
        Step::together([Animation::new(&b, 1.0).with_options(linear(100))]),
      ]
    };
    let playback = component.play_with(steps(), PlayOptions::default().with_looping(true));
    let frame = |millis: u64| component.clone().render(Duration::from_millis(millis));
    frame(3000);
    for lap in 0..3 {
      frame(16);
      frame(100); // step one arrives
      frame(1); // step two starts
      frame(16);
      frame(100); // step two arrives
      assert_eq!((a.peek(), b.peek()), (10.0, 1.0), "lap {lap} reaches the end");
      frame(1); // the sequence notices, and starts again
      assert_eq!((a.peek(), b.peek()), (0.0, 0.0), "lap {lap} puts the properties back");
      assert!(!playback.finished().peek(), "a looping sequence never finishes by itself");
      frame(1); // the first step begins again
    }

    playback.looping().next(false); // from now on it plays to the end once
    frame(16);
    frame(100);
    frame(1);
    frame(16);
    frame(100);
    frame(1);
    assert!(playback.finished().peek());
    assert_eq!((a.peek(), b.peek()), (10.0, 1.0));

    // A loop is ended by stop.
    let endless = component.play_with(steps(), PlayOptions::default().with_looping(true));
    frame(1);
    endless.stop();
    frame(16);
    assert!(endless.finished().peek());
    assert_eq!(component.clone().pacing(), Pacing::OnDemand);
  }

  #[test]
  fn restart_puts_the_properties_back_and_plays_again_whether_it_is_running_or_over() {
    let component = Component::new("sequence").with_size(2, 2);
    let (a, b) = (BehaviorSubject::new(0.0f32), BehaviorSubject::new(0.0f32));
    let playback = component.play([
      Step::together([Animation::new(&a, 10.0).with_options(linear(100))]),
      Step::together([Animation::new(&b, 1.0).with_options(linear(100))]).with_delay(Duration::from_millis(50)),
    ]);
    let frame = |millis: u64| component.clone().render(Duration::from_millis(millis));
    frame(3000);
    frame(16);
    frame(50);
    assert_eq!(a.peek(), 5.0);

    playback.restart(); // while it is running
    frame(16); // it puts itself back and starts step one again
    assert_eq!((a.peek(), b.peek()), (0.0, 0.0));
    frame(16);
    frame(50);
    assert_eq!(a.peek(), 5.0, "it ran again from the start");

    // Play it through to the end.
    frame(60);
    frame(10);
    frame(60);
    frame(16);
    frame(100);
    frame(1);
    assert_eq!((a.peek(), b.peek(), playback.finished().peek()), (10.0, 1.0, true));
    assert_eq!(component.clone().pacing(), Pacing::OnDemand);

    playback.restart(); // after it is over
    assert_eq!((a.peek(), b.peek()), (0.0, 0.0), "put back at once");
    assert!(!playback.finished().peek());
    assert_eq!(component.clone().pacing(), Pacing::Continuous);
    frame(1);
    frame(16);
    frame(50);
    assert_eq!(a.peek(), 5.0);
  }
}
