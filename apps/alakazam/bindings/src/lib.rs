use napi::bindgen_prelude::*;
use napi_derive::napi;

pub mod adjustments;
pub mod filters;
pub mod generate_image;
pub mod layer;
pub mod metadata;
pub mod project;

pub(crate) mod common {
  pub use crate::ImageData;
  pub use crate::layer::Layer;
  pub use crate::metadata::{LayerMetadata, ProjectMetadata};
  pub use crate::project::Project;
  pub use abra::filters::prelude::{noise::NoiseDistribution, *};
  pub use napi::bindgen_prelude::Buffer;
  pub use napi_derive::napi;
}

/// Result of opening an image, includes dimensions for canvas rendering.
#[napi(object, js_name = "AbraImageData")]
pub struct ImageData {
  pub data: Buffer,
  pub width: u32,
  pub height: u32,
}
