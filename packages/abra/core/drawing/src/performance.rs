use abra_core::{Performance, Settings};

/// Rendering algorithm used by drawing operations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawingAlgorithm {
  /// Select the fastest algorithm that preserves the selected profile's quality policy.
  #[default]
  Auto,
  /// Use the general supersampled rasterizer for every shape.
  Rasterizer,
  /// Use analytic signed-distance rendering when the shape and fill support it.
  SignedDistanceField,
}

#[derive(Clone, Copy)]
pub(crate) struct ResolvedPerformance {
  pub algorithm: DrawingAlgorithm,
  pub anti_aliasing: u32,
}

impl ResolvedPerformance {
  pub fn from_performance(p_performance: Performance<DrawingAlgorithm>) -> Self {
    match p_performance {
      Performance::Fast => Self {
        algorithm: DrawingAlgorithm::Auto,
        anti_aliasing: 1,
      },
      Performance::Balanced => Self {
        algorithm: DrawingAlgorithm::Auto,
        anti_aliasing: 2,
      },
      Performance::Quality => Self {
        algorithm: DrawingAlgorithm::Rasterizer,
        anti_aliasing: 4,
      },
      Performance::Algorithm(algorithm) => Self {
        algorithm,
        anti_aliasing: 2,
      },
      Performance::Custom(options) => Self {
        algorithm: *options.algorithm(),
        anti_aliasing: options.anti_aliasing() as u32,
      },
    }
  }
}

pub(crate) fn configured_performance() -> Performance<DrawingAlgorithm> {
  match Settings::drawing_performance().to_ascii_lowercase().as_str() {
    "fast" => Performance::Fast,
    "quality" | "high-quality" | "high_quality" => Performance::Quality,
    _ => Performance::Balanced,
  }
}

#[cfg(test)]
mod tests {
  use super::{DrawingAlgorithm, ResolvedPerformance};
  use abra_core::{Performance, PerformanceOptions};

  #[test]
  fn balanced_profile_uses_two_by_two_sampling() {
    let resolved = ResolvedPerformance::from_performance(Performance::Balanced);
    assert_eq!(resolved.anti_aliasing, 2);
    assert_eq!(resolved.algorithm, DrawingAlgorithm::Auto);
  }

  #[test]
  fn custom_profile_preserves_its_configuration() {
    let resolved = ResolvedPerformance::from_performance(Performance::Custom(
      PerformanceOptions::new(DrawingAlgorithm::Rasterizer).with_anti_aliasing(4),
    ));
    assert_eq!(resolved.anti_aliasing, 4);
    assert_eq!(resolved.algorithm, DrawingAlgorithm::Rasterizer);
  }
}
