//! What the editor's controls look like while they are being tried out.
//!
//! React tells a `LiveView` only which control was touched and its value (`SliderMove`) or which action was tapped
//! (`Action`). The effects those controls stand for are described here, once, so the screens do not repeat them.

use abra::adjustments::prelude::levels::FilterType;

use crate::effect_spec::EffectSpec;

/// The effects a slider shows at `p_value`, or `None` if the key is not a slider with a live form.
pub fn slider(p_key: &str, p_value: f64) -> Option<Vec<EffectSpec>> {
  let whole = p_value.round();
  Some(vec![match p_key {
    "action-brightness" => EffectSpec::Brightness { amount: whole as i32 },
    "action-contrast" => EffectSpec::Contrast { amount: whole },
    "action-saturation" => EffectSpec::Saturation { amount: whole as i32 },
    "action-exposure" => EffectSpec::Exposure {
      exposure: p_value,
      offset: 0.0,
      gamma_correction: 1.0,
    },
    "action-vibrance" => EffectSpec::Vibrance {
      vibrance: whole,
      saturation: 0.0,
    },
    "action-gaussian-blur" => EffectSpec::GaussianBlur { radius: p_value },
    "action-threshold" => EffectSpec::Threshold {
      amount: p_value.clamp(0.0, 255.0) as u8,
    },
    "action-skin-smooth" => EffectSpec::SkinSmooth { amount: p_value },
    "action-skin-tan" => EffectSpec::SkinTan { offset: p_value },
    "action-skin-tone" => EffectSpec::SkinTone { amount: p_value },
    _ => return None,
  }])
}

fn filter(p_filter: FilterType, p_density: f64) -> EffectSpec {
  EffectSpec::PhotoFilter {
    filter: Some(p_filter),
    density: p_density,
    preserve_luminosity: false,
    color: None,
  }
}

/// The effects an action shows, in the order they run, or `None` if the key is not an action with a live form. The empty
/// key shows the image as it is.
pub fn action(p_key: &str) -> Option<Vec<EffectSpec>> {
  use EffectSpec::{Brightness, Contrast, Grayscale, Invert, Saturation, Vibrance};
  Some(match p_key {
    "" => vec![],
    "action-grayscale" => vec![Grayscale],
    "action-invert" => vec![Invert],
    "action-mood-vibrant" => vec![
      Saturation { amount: 30 },
      Vibrance {
        vibrance: 30.0,
        saturation: 0.0,
      },
    ],
    "action-mood-warm" => vec![filter(FilterType::WarmingLight, 0.35)],
    "action-mood-golden-hour" => vec![filter(FilterType::WarmingLight, 0.45), Brightness { amount: 12 }],
    "action-mood-vintage" => vec![
      filter(FilterType::Sepia, 0.2),
      Contrast { amount: -10.0 },
      Saturation { amount: -20 },
    ],
    "action-mood-noir" => vec![Saturation { amount: -100 }, Contrast { amount: 20.0 }],
    "action-mood-moody" => vec![
      filter(FilterType::CoolingLight, 0.25),
      Contrast { amount: 15.0 },
      Brightness { amount: -10 },
    ],
    "action-mood-sunny" => vec![
      filter(FilterType::WarmingLight, 0.2),
      Vibrance {
        vibrance: 20.0,
        saturation: 0.0,
      },
      Brightness { amount: 8 },
    ],
    "action-mood-cold" => vec![filter(FilterType::CoolingLight, 0.35)],
    "action-mood-fade" => vec![Contrast { amount: -20.0 }, Saturation { amount: -15 }],
    "action-mood-sepia" => vec![filter(FilterType::Sepia, 0.35)],
    "action-mood-dreamy" => vec![
      filter(FilterType::WarmingLight, 0.15),
      Contrast { amount: -15.0 },
      Saturation { amount: -10 },
    ],
    "action-mood-cloudy" => vec![
      filter(FilterType::CoolingLight, 0.2),
      Saturation { amount: -20 },
      Brightness { amount: -6 },
    ],
    "action-mood-midnight" => vec![
      filter(FilterType::CoolingDark, 0.4),
      Brightness { amount: -20 },
      Contrast { amount: 15.0 },
      Saturation { amount: -10 },
    ],
    "action-mood-ocean" => vec![filter(FilterType::Underwater, 0.35)],
    "action-mood-forest" => vec![filter(FilterType::DeepEmerald, 0.3)],
    "action-mood-frost" => vec![filter(FilterType::CoolingDark, 0.3)],
    "action-mood-neon" => vec![filter(FilterType::Magenta, 0.25)],
    _ => return None,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sliders_and_actions_map_to_effects_and_unknown_keys_to_nothing() {
    assert_eq!(slider("action-brightness", 49.6), Some(vec![EffectSpec::Brightness { amount: 50 }]));
    for amount in [-100.0, 0.0, 100.0] {
      assert_eq!(slider("action-skin-tone", amount), Some(vec![EffectSpec::SkinTone { amount }]));
    }
    assert_eq!(slider("action-nothing", 1.0), None);
    assert_eq!(action("action-mood-noir").map(|effects| effects.len()), Some(2));
    assert_eq!(action(""), Some(vec![]));
    assert_eq!(action("action-nothing"), None);
  }
}
