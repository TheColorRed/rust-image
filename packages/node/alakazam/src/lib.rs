#![allow(dead_code)]
#![allow(mismatched_lifetime_syntaxes)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

pub mod adjustments;
pub mod apply_options;
pub mod area;
pub mod color;
pub mod filters;
pub mod generate_image;
pub mod gradient;
pub mod image_data;
pub mod layer;
pub mod live;
pub mod metadata;
pub mod path;
pub mod project;

pub mod gizmos;
pub mod history;

pub(crate) mod common {
  pub use crate::ImageData;
  pub use crate::apply_options::ApplyOptions;
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

impl ImageData {
  pub fn from_image(img: &abra::abra_core::Image) -> Self {
    let (width, height) = img.dimensions::<u32>();
    ImageData {
      width,
      height,
      data: Buffer::from(img.rgba()),
    }
  }
}

impl Clone for ImageData {
  fn clone(&self) -> Self {
    let data = self.data.to_vec();
    let buffer = napi::bindgen_prelude::Buffer::from(data);
    Self {
      data: buffer,
      width: self.width,
      height: self.height,
    }
  }
}

impl From<abra::abra_core::Image> for ImageData {
  fn from(img: abra::abra_core::Image) -> Self {
    let (width, height) = img.dimensions::<u32>();
    ImageData {
      width,
      height,
      data: Buffer::from(img.rgba()),
    }
  }
}

#[napi]
#[derive(Clone)]
pub struct Mask {
  pub(crate) inner: abra::mask::prelude::Mask,
}
