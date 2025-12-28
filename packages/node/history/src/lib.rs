pub mod history;

/// Result of opening an image, includes dimensions for canvas rendering.
#[napi_derive::napi(object, js_name = "AbraImageData")]
pub struct ImageData {
  pub data: napi::bindgen_prelude::Buffer,
  pub width: u32,
  pub height: u32,
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

pub(crate) mod common {
  pub use crate::ImageData;
  pub use napi::bindgen_prelude::Buffer;
  pub use napi_derive::napi;
}
