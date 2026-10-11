/// Selects a performance preset, a module-specific algorithm, or explicit settings.
///
/// `Balanced` is the default and should provide a measured compromise between
/// execution speed and image quality. Each module defines its own algorithm
/// type and resolves this selection once before entering its hot path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Performance<A> {
  /// Prioritize throughput where the module provides a documented faster path.
  Fast,
  /// Use the module's balanced speed and quality defaults.
  Balanced,
  /// Prioritize output quality where the module provides a documented quality path.
  Quality,
  /// Use a module-specific algorithm with that algorithm's default settings.
  Algorithm(A),
  /// Use a module-specific algorithm with explicit tuning values.
  Custom(PerformanceOptions<A>),
}

impl<A> Default for Performance<A> {
  fn default() -> Self {
    Self::Balanced
  }
}

/// Explicit performance configuration for a module-specific algorithm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceOptions<A> {
  algorithm: A,
  anti_aliasing: u8,
}

impl<A> PerformanceOptions<A> {
  /// Creates explicit settings for `p_algorithm` using balanced antialiasing.
  pub fn new(p_algorithm: A) -> Self {
    Self {
      algorithm: p_algorithm,
      anti_aliasing: 2,
    }
  }

  /// Sets the number of samples per pixel side, clamped to the supported range.
  pub fn with_anti_aliasing(mut self, p_samples_per_side: u8) -> Self {
    self.anti_aliasing = p_samples_per_side.clamp(1, 16);
    self
  }

  /// Returns the selected module-specific algorithm.
  pub fn algorithm(&self) -> &A {
    &self.algorithm
  }

  /// Returns the number of antialiasing samples per pixel side.
  pub fn anti_aliasing(&self) -> u8 {
    self.anti_aliasing
  }
}

#[cfg(test)]
mod tests {
  use super::{Performance, PerformanceOptions};

  #[derive(Clone, Debug, PartialEq, Eq)]
  enum Algorithm {
    Scanline,
  }

  #[test]
  fn defaults_to_balanced() {
    assert_eq!(Performance::<Algorithm>::default(), Performance::Balanced);
  }

  #[test]
  fn custom_options_clamp_antialiasing() {
    let options = PerformanceOptions::new(Algorithm::Scanline).with_anti_aliasing(0);
    assert_eq!(options.anti_aliasing(), 1);
    assert_eq!(options.algorithm(), &Algorithm::Scanline);
  }
}
