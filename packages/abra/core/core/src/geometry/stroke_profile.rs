//! What a stroke's width and opacity have in common: a value set at stops along the path, where `0` is its start and `1` is its end.

/// How the value changes between two stops of a [`StrokeWidth`](crate::StrokeWidth) or a [`StrokeOpacity`](crate::StrokeOpacity).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokeEasing {
  /// The value changes at the same rate from one stop to the next.
  #[default]
  Linear,
  /// The value eases out of one stop and into the next (smoothstep), so a swell settles without a visible bend at the stops.
  Smooth,
}

/// A value set at one place along a stroke.
pub(crate) trait Stop {
  /// What is set at the stop.
  fn value(&self) -> f32;
  /// How far along the path the stop is, from `0` at its start to `1` at its end.
  fn time(&self) -> f32;
}

/// The value `p_time` of the way along a stroke from stops that are sorted by their time, or nothing when there are none. Before the first
/// stop and after the last it is that stop's value, and between two stops it changes from one to the other as `p_easing` says.
pub(crate) fn value_at<S: Stop>(p_stops: &[S], p_easing: StrokeEasing, p_time: f32) -> Option<f32> {
  let after = p_stops.iter().position(|stop| stop.time() > p_time);
  Some(match after {
    Some(0) => p_stops[0].value(),
    Some(index) => {
      let (start, end) = (&p_stops[index - 1], &p_stops[index]);
      let t = ((p_time - start.time()) / (end.time() - start.time())).clamp(0.0, 1.0);
      let t = match p_easing {
        StrokeEasing::Linear => t,
        StrokeEasing::Smooth => t * t * (3.0 - 2.0 * t),
      };
      start.value() + (end.value() - start.value()) * t
    }
    None => p_stops.last()?.value(),
  })
}
