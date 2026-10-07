use crate::Image;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Marker trait implemented by owners that need to be kept alive while a
/// `ImageRef` exists. Implementations can be provided by other crates (e.g. Canvas).
pub trait GuardedOwner {}

/// A lightweight reference wrapper that gives mutable access to an Image and
/// optionally owns an opaque owner that keeps a mutex/guard alive for the duration
/// of the `ImageRef`.
pub struct ImageRef<'a> {
  ptr: *mut Image,
  _owner: Option<Box<dyn GuardedOwner + 'a>>,
  _marker: PhantomData<&'a mut Image>,
}

impl<'a> ImageRef<'a> {
  pub fn new(p_ptr: *mut Image, p_owner: Option<Box<dyn GuardedOwner + 'a>>) -> Self {
    Self {
      ptr: p_ptr,
      _owner: p_owner,
      _marker: PhantomData,
    }
  }
}

impl<'a> Deref for ImageRef<'a> {
  type Target = Image;
  fn deref(&self) -> &Image {
    unsafe { &*self.ptr }
  }
}

impl<'a> DerefMut for ImageRef<'a> {
  fn deref_mut(&mut self) -> &mut Image {
    unsafe { &mut *self.ptr }
  }
}

impl<'a> From<&'a mut Image> for ImageRef<'a> {
  fn from(p_image: &'a mut Image) -> Self {
    let ptr = p_image as *mut Image;
    ImageRef::new(ptr, None)
  }
}

impl Into<Image> for ImageRef<'static> {
  fn into(self) -> Image {
    unsafe { std::ptr::read(&*self as *const Image) }
  }
}
