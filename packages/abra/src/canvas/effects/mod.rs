//! Effects module for applying effects to layers

/// Drop shadow effect implementation.
pub(crate) mod drop_shadow;
/// Stroke effect implementation.
pub(crate) mod stroke;

/// Drop shadow effect options.
pub(crate) mod options_drop_shadow;
/// Outline effect options.
pub(crate) mod options_stroke;

pub use options_drop_shadow::DropShadowOptions;
pub use options_stroke::StrokeOptions;
