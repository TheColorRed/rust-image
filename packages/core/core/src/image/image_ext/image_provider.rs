use primitives::Image as PrimitiveImage;

/// Helper trait for things that can provide mutable access to a `PrimitiveImage`.
///
/// This trait enables API consumers (like filters) to accept either a plain
/// `Image` or a higher-level wrapper (e.g. `Layer`) that can provide access to
/// an `Image` without introducing an additional circular dependency.
pub trait WithImageMut {
  /// Execute the provided closure with a mutable borrow of the image.
  fn with_image_mut<R>(&mut self, f: impl FnOnce(&mut PrimitiveImage) -> R) -> R;
}

impl WithImageMut for PrimitiveImage {
  fn with_image_mut<R>(&mut self, f: impl FnOnce(&mut PrimitiveImage) -> R) -> R {
    f(self)
  }
}
