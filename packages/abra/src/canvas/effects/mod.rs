//! Effects module for applying effects to layers

/// Drop shadow effect implementation.
pub(crate) mod drop_shadow;
/// Stroke effect implementation.
pub(crate) mod stroke;

/// Drop shadow effect options.
pub(crate) mod options_drop_shadow;
/// Outline effect options.
pub(crate) mod options_stroke;

pub use drop_shadow::drop_shadow;
pub use options_drop_shadow::DropShadowOptions;
pub use options_stroke::StrokeOptions;
pub use stroke::stroke;

/// Trait for applying shadow effects to layers.
pub trait Shadow {
  /// Applies a drop shadow effect to the layer.
  fn drop_shadow(&self, options: DropShadowOptions);
}

/// Trait for applying stroke effects to layers.
pub trait Stroke {
  /// Applies an outline effect to the layer.
  fn stroke(&self, options: StrokeOptions);
}
