use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

pub mod color_picker;
pub mod cursors;
pub mod layer_tools;

pub(crate) mod common {
  pub use crate::ImageData;
  pub use abra::prelude::*;
  pub use napi_derive::napi;
}

#[napi(object, js_name = "AbraGizmosImageData")]
pub struct ImageData {
  pub data: Buffer,
  pub width: u32,
  pub height: u32,
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
