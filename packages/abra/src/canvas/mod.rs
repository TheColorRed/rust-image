//! Canvas management and layer composition.

/// Anchor points for positioning elements.
pub mod anchor;
/// The Canvas public API struct.
pub mod canvas;
/// The internal canvas implementation.
pub mod canvas_inner;
/// Transform operations for canvases.
pub mod canvas_transform;
/// The Layer public API struct.
pub mod layer;
/// The internal layer implementation.
pub mod layer_inner;
/// Utilities for applying layer options.
pub mod layer_options_applier;
/// Utilities for applying layer size options.
pub mod layer_size_applier;
/// Transform operations for layers.
pub mod layer_transform;
/// Options for adding a canvas to another canvas.
pub mod options_add_canvas;
/// Options for creating a new layer in a canvas.
pub mod options_new_layer;

pub use anchor::Anchor;
pub use canvas::Canvas;
pub use canvas_transform::CanvasTransform;
pub use layer::Layer;
pub use layer_transform::LayerTransform;
pub use options_add_canvas::AddCanvasOptions;
pub use options_new_layer::{LayerSize, NewLayerOptions};
