//! Canvas management and layer composition.

mod anchor;
mod canvas;
pub(crate) mod canvas_inner;
mod canvas_transform;
mod layer;
pub(crate) mod layer_inner;
mod layer_options_applier;
mod layer_size_applier;
mod layer_transform;
mod options_add_canvas;
mod options_new_layer;
mod origin;

pub use anchor::Anchor;
pub use canvas::Canvas;
pub use canvas_transform::CanvasTransform;
pub use layer::Layer;
pub use layer_transform::LayerTransform;
pub use options_add_canvas::AddCanvasOptions;
pub use options_new_layer::{LayerSize, NewLayerOptions};
pub use origin::Origin;
