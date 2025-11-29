use primitives::Image as PrimitiveImage;

use crate::Size;

pub trait CoreImageSizeExt {
  /// Get the size of the image as a `Size` struct.
  fn size(&self) -> Size;
}

impl CoreImageSizeExt for PrimitiveImage {
  fn size(&self) -> Size {
    let (width, height) = self.dimensions::<i32>();
    Size {
      width: width as f32,
      height: height as f32,
    }
  }
}
