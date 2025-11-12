use crate::canvas::{StrokeOptions, effects::options_drop_shadow::DropShadowOptions};

#[derive(Clone)]
/// Options for various layer effects.
pub struct LayerEffectOptions {
  /// Options for drop shadow effect.
  pub drop_shadow: Option<DropShadowOptions>,
  /// Options for stroke effect.
  pub stroke: Option<StrokeOptions>,
}

impl LayerEffectOptions {
  /// Creates a new LayerEffectOptions with default settings.
  pub fn new() -> Self {
    LayerEffectOptions {
      drop_shadow: None,
      stroke: None,
    }
  }

  /// Configures the drop shadow effect.
  pub fn with_drop_shadow(mut self, options: DropShadowOptions) -> Self {
    self.drop_shadow = Some(options);
    self
  }

  /// Configures the stroke effect.
  pub fn with_stroke(mut self, options: StrokeOptions) -> Self {
    self.stroke = Some(options);
    self
  }
}
