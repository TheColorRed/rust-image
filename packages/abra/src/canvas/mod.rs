//! Canvas management and layer composition.

/// Anchor points for positioning elements.
mod anchor;
/// The Canvas public API struct.
mod canvas;
/// The internal canvas implementation.
mod canvas_inner;
/// Transform operations for canvases.
mod canvas_transform;
/// Effects that can be applied to layers.
mod effects;
/// The Layer public API struct.
mod layer;
/// Effects that can be applied to layers.
mod layer_effects;
/// The internal layer implementation.
mod layer_inner;
/// Utilities for applying layer options.
mod layer_options_applier;
/// Utilities for applying layer size options.
mod layer_size_applier;
/// Layer transform operations.
mod layer_transform;
/// Options for adding a canvas to another canvas.
mod options_add_canvas;
/// Options for applying effects to layers.
mod options_layer_effects;
/// Options for creating a new layer in a canvas.
mod options_new_layer;
/// Origin points for layer positioning.
mod origin;

pub use anchor::Anchor;
pub use canvas::Canvas;
pub use canvas_transform::CanvasTransform;
pub use effects::{DropShadowOptions, Shadow, Stroke, StrokeOptions, drop_shadow, stroke};
pub use layer::Layer;
pub use layer_effects::LayerEffects;
pub use layer_transform::LayerTransform;
pub use options_add_canvas::AddCanvasOptions;
pub use options_layer_effects::LayerEffectOptions;
pub use options_new_layer::{LayerSize, NewLayerOptions};
pub use origin::Origin;
