use crate::canvas::{StrokeOptions, effects::options_drop_shadow::DropShadowOptions};
use std::sync::Arc;

#[derive(Clone)]
/// Options for various layer effects.
pub struct LayerEffects {
  /// Options for drop shadow effect.
  pub drop_shadow: Option<DropShadowOptions>,
  /// Options for stroke effect.
  pub stroke: Option<StrokeOptions>,
  /// Internal: reference to apply effects to (used when chaining from layer.effects())
  pub(crate) layer_inner: Option<Arc<std::sync::Mutex<crate::canvas::layer_inner::LayerInner>>>,
}

impl LayerEffects {
  /// Creates a new LayerEffects with default settings.
  pub fn new() -> Self {
    LayerEffects {
      drop_shadow: None,
      stroke: None,
      layer_inner: None,
    }
  }

  /// Configures the drop shadow effect.
  pub fn with_drop_shadow(mut self, options: DropShadowOptions) -> Self {
    self.drop_shadow = Some(options);
    self.apply_to_layer();
    self
  }

  /// Configures the stroke effect.
  pub fn with_stroke(mut self, options: StrokeOptions) -> Self {
    self.stroke = Some(options);
    self.apply_to_layer();
    self
  }

  /// Internal: applies current options to the layer if one is set.
  fn apply_to_layer(&self) {
    if let Some(ref layer_inner) = self.layer_inner {
      let effects = self.build_effects();
      layer_inner.lock().unwrap().set_effects_builder(effects);
    }
  }

  /// Internal: builds the effects from the current options.
  fn build_effects(&self) -> EffectsBuilder {
    let mut builder = EffectsBuilder::new();

    if let Some(ref stroke_opts) = self.stroke {
      builder = builder.with_stroke(stroke_opts.clone());
    }
    if let Some(ref shadow_opts) = self.drop_shadow {
      builder = builder.with_drop_shadow(shadow_opts.clone());
    }

    builder
  }

  /// Internal: sets the layer reference for auto-application.
  pub(crate) fn with_layer(mut self, layer: Arc<std::sync::Mutex<crate::canvas::layer_inner::LayerInner>>) -> Self {
    self.layer_inner = Some(layer);
    self
  }
}

/// Internal builder for queuing and applying effects to an image.
/// Effects are applied in the order they are added.
#[derive(Clone, Debug)]
pub(crate) struct EffectsBuilder {
  effects: Vec<Effect>,
}

#[derive(Clone, Debug)]
enum Effect {
  Stroke(StrokeOptions),
  DropShadow(DropShadowOptions),
}

impl EffectsBuilder {
  /// Creates a new empty effects builder.
  pub(crate) fn new() -> Self {
    Self { effects: Vec::new() }
  }

  /// Adds a stroke effect to be applied.
  pub(crate) fn with_stroke(mut self, options: StrokeOptions) -> Self {
    self.effects.push(Effect::Stroke(options));
    self
  }

  /// Adds a drop shadow effect to be applied.
  pub(crate) fn with_drop_shadow(mut self, options: DropShadowOptions) -> Self {
    self.effects.push(Effect::DropShadow(options));
    self
  }

  /// Applies all queued effects to the given image and returns the modified image.
  pub(crate) fn apply(&self, mut image: Arc<crate::Image>) -> Arc<crate::Image> {
    for effect in &self.effects {
      match effect {
        Effect::Stroke(options) => {
          image = Self::apply_stroke(image, options);
        }
        Effect::DropShadow(options) => {
          image = Self::apply_drop_shadow(image, options);
        }
      }
    }
    image
  }

  fn apply_stroke(image: Arc<crate::Image>, options: &StrokeOptions) -> Arc<crate::Image> {
    use crate::canvas::effects::options_stroke::OutlinePosition;
    use crate::draw::line;
    use crate::geometry::path::Path;
    use crate::geometry::point::Point;
    use crate::utils::debug::DebugEffects;
    use std::time::Instant;

    let duration = Instant::now();
    let original_image = image.as_ref();
    let (width, height) = original_image.dimensions::<u32>();

    let mut composite = crate::Image::new(width, height);
    composite.set_rgba(original_image.rgba());

    let start = match options.position {
      OutlinePosition::Inside => Point::new(options.size as i32 / 2, options.size as i32 / 2),
      OutlinePosition::Outside => Point::new(-(options.size as i32 / 2), -(options.size as i32 / 2)),
      OutlinePosition::Center => Point::new(0, 0),
    };

    let stroke_max_width = match options.position {
      OutlinePosition::Inside => width - options.size,
      OutlinePosition::Outside => width + options.size,
      OutlinePosition::Center => 0,
    };

    let stroke_max_height = match options.position {
      OutlinePosition::Inside => height - options.size,
      OutlinePosition::Outside => height + options.size,
      OutlinePosition::Center => 0,
    };

    let relative_path = vec![
      Point::new(0, 0),
      Point::new((stroke_max_width) as i32, 0),
      Point::new((stroke_max_width) as i32, (stroke_max_height) as i32),
      Point::new(0, (stroke_max_height) as i32),
      Point::new(0, 0),
    ];

    let cap = match options.position {
      OutlinePosition::Inside => line::LineCap::Square,
      OutlinePosition::Outside => line::LineCap::Round,
      OutlinePosition::Center => line::LineCap::Round,
    };

    let fill = options.fill.clone();
    line::line(&mut composite, start, Path::new(relative_path), fill, options.size, Some(cap));

    DebugEffects::Stroke(options.clone(), duration.elapsed()).log();

    Arc::new(composite)
  }

  fn apply_drop_shadow(_image: Arc<crate::Image>, _options: &DropShadowOptions) -> Arc<crate::Image> {
    // TODO: Implement drop shadow application here
    _image
  }
}

impl Default for EffectsBuilder {
  fn default() -> Self {
    Self::new()
  }
}
