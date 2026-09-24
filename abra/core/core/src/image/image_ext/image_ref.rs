use primitives::Image as PrimitiveImage;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Marker trait implemented by owners that need to be kept alive while a
/// `ImageRef` exists. Implementations can be provided by other crates (e.g. Canvas).
pub trait GuardedOwner {}

/// A lightweight reference wrapper that gives mutable access to an Image and
/// optionally owns an opaque owner that keeps a mutex/guard alive for the duration
/// of the `ImageRef`.
pub struct ImageRef<'a> {
  ptr: *mut PrimitiveImage,
  _owner: Option<Box<dyn GuardedOwner + 'a>>,
  _marker: PhantomData<&'a mut PrimitiveImage>,
}

impl<'a> ImageRef<'a> {
  pub fn new(p_ptr: *mut PrimitiveImage, p_owner: Option<Box<dyn GuardedOwner + 'a>>) -> Self {
    Self {
      ptr: p_ptr,
      _owner: p_owner,
      _marker: PhantomData,
    }
  }
}

impl<'a> Deref for ImageRef<'a> {
  type Target = PrimitiveImage;
  fn deref(&self) -> &PrimitiveImage {
    unsafe { &*self.ptr }
  }
}

impl<'a> DerefMut for ImageRef<'a> {
  fn deref_mut(&mut self) -> &mut PrimitiveImage {
    unsafe { &mut *self.ptr }
  }
}

impl<'a> From<&'a mut PrimitiveImage> for ImageRef<'a> {
  fn from(p_image: &'a mut PrimitiveImage) -> Self {
    let ptr = p_image as *mut PrimitiveImage;
    ImageRef::new(ptr, None)
  }
}
