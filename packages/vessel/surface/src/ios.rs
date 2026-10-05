//! The C bridge for a Metal layer owned by an iOS host. The host retains its renderer until this surface is dropped.

use std::any::Any;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use crate::{Surface, Touch, register_surface};

type Draw = unsafe extern "C" fn(*mut c_void, u32, u32, *const u8, usize, u32, u32) -> bool;
type Release = unsafe extern "C" fn(*mut c_void);

struct State {
  alive: bool,
  size: (u32, u32),
}

/// An iOS host's retained Metal layer, usable by a GPU presenter or the host's pixel uploader.
pub struct IosSurface {
  layer: *mut c_void,
  context: *mut c_void,
  draw: Draw,
  release: Release,
  state: Mutex<State>,
}

// The host's renderer owns the layer; access is serialized by state, and release may run on the rendering thread.
unsafe impl Send for IosSurface {}
unsafe impl Sync for IosSurface {}

impl IosSurface {
  /// Serializes a GPU presentation against resize and retirement. An old surface cannot draw after replacement.
  pub fn with_layer(&self, p_draw: impl FnOnce(*mut c_void, (u32, u32)) -> bool) -> bool {
    let state = self.state.lock().unwrap();
    state.alive && p_draw(self.layer, state.size)
  }
}

impl Drop for IosSurface {
  fn drop(&mut self) {
    // SAFETY: registration transferred exactly one retained context, and no draw can outlive this surface.
    unsafe { (self.release)(self.context) };
  }
}

impl Surface for IosSurface {
  fn is_alive(&self) -> bool {
    self.state.lock().unwrap().alive
  }

  fn size(&self) -> (u32, u32) {
    self.state.lock().unwrap().size
  }

  fn resize(&self, p_width: u32, p_height: u32) {
    self.state.lock().unwrap().size = (p_width, p_height);
  }

  fn draw_rgba(&self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> bool {
    let expected = (p_width as usize).checked_mul(p_height as usize).and_then(|size| size.checked_mul(4));
    if p_width == 0 || p_height == 0 || expected != Some(p_rgba.len()) {
      eprintln!("vessel: invalid iOS frame dimensions or RGBA length");
      return false;
    }
    self.with_layer(|_, (width, height)| {
      // SAFETY: context is retained, the slice lives until the synchronous callback returns, and drawing is locked.
      unsafe { (self.draw)(self.context, p_width, p_height, p_rgba.as_ptr(), p_rgba.len(), width, height) }
    })
  }

  fn retire(&self) {
    self.state.lock().unwrap().alive = false;
  }

  fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
    self
  }
}

/// Registers an iOS host's Metal surface. On success this owns `p_context` and eventually calls `p_release` once.
///
/// # Safety
/// `p_context` must retain `p_layer` and everything needed by both callbacks. The callbacks must be thread-safe, never
/// unwind, and copy any borrowed pixel data before returning. On failure the caller still owns the context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vessel_ios_surface_available(
  p_id: i32, p_layer: *mut c_void, p_context: *mut c_void, p_draw: Option<Draw>, p_release: Option<Release>,
  p_width: u32, p_height: u32,
) -> bool {
  let (Some(draw), Some(release)) = (p_draw, p_release) else {
    eprintln!("vessel: iOS surface callbacks are required");
    return false;
  };
  if p_id < 0 || p_layer.is_null() || p_context.is_null() || p_width == 0 || p_height == 0 {
    eprintln!("vessel: invalid iOS surface registration");
    return false;
  }
  register_surface(
    p_id,
    Arc::new(IosSurface {
      layer: p_layer,
      context: p_context,
      draw,
      release,
      state: Mutex::new(State {
        alive: true,
        size: (p_width, p_height),
      }),
    }),
  );
  true
}

/// Updates an iOS view's drawable size in physical pixels.
#[unsafe(no_mangle)]
pub extern "C" fn vessel_ios_surface_resized(p_id: i32, p_width: u32, p_height: u32) {
  crate::resize(p_id, p_width, p_height);
}

/// Retires an iOS view's surface before its host releases the view.
#[unsafe(no_mangle)]
pub extern "C" fn vessel_ios_surface_destroyed(p_id: i32) {
  crate::destroy(p_id);
}

/// Forwards an iOS touch in physical pixels: 0 is down, 1 is moved, and 2 is up or cancelled.
#[unsafe(no_mangle)]
pub extern "C" fn vessel_ios_touch(p_id: i32, p_action: i32, p_x: f32, p_y: f32) {
  let touch = match p_action {
    0 => Touch::Down,
    1 => Touch::Moved,
    2 => Touch::Up,
    _ => {
      eprintln!("vessel: invalid iOS touch action {p_action}");
      return;
    }
  };
  crate::touch(p_id, touch, p_x, p_y);
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use super::*;
  use crate::{destroy, draw_rgba, get_as, resize};

  #[derive(Default)]
  struct Recorder {
    frames: Mutex<Vec<(Vec<u8>, u32, u32)>>,
    released: AtomicUsize,
  }

  unsafe extern "C" fn draw(
    p_context: *mut c_void, _w: u32, _h: u32, p_pixels: *const u8, p_len: usize, p_w: u32, p_h: u32,
  ) -> bool {
    let recorder = unsafe { &*p_context.cast::<Recorder>() };
    recorder.frames.lock().unwrap().push((unsafe { std::slice::from_raw_parts(p_pixels, p_len) }.to_vec(), p_w, p_h));
    true
  }

  unsafe extern "C" fn release(p_context: *mut c_void) {
    let recorder = unsafe { Arc::from_raw(p_context.cast::<Recorder>()) };
    recorder.released.fetch_add(1, Ordering::SeqCst);
  }

  fn register(p_id: i32, p_recorder: &Arc<Recorder>) {
    let context = Arc::into_raw(p_recorder.clone()).cast_mut().cast();
    assert!(unsafe { vessel_ios_surface_available(p_id, context, context, Some(draw), Some(release), 2, 2) });
  }

  #[test]
  fn pixels_resize_retirement_and_retained_ownership() {
    let recorder = Arc::new(Recorder::default());
    register(9201, &recorder);
    let surface = get_as::<IosSurface>(9201).unwrap();
    assert!(draw_rgba(9201, 1, 1, &[1, 2, 3, 4]));
    resize(9201, 4, 6);
    assert!(draw_rgba(9201, 1, 1, &[5, 6, 7, 8]));
    assert_eq!(recorder.frames.lock().unwrap()[1], (vec![5, 6, 7, 8], 4, 6));
    destroy(9201);
    assert!(!surface.draw_rgba(1, 1, &[0; 4]));
    assert!(!surface.with_layer(|_, _| panic!("retired surface must not present")));
    assert_eq!(recorder.released.load(Ordering::SeqCst), 0);
    drop(surface);
    assert_eq!(recorder.released.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn replacement_retires_old_layer_and_releases_each_context_once() {
    let first = Arc::new(Recorder::default());
    let second = Arc::new(Recorder::default());
    register(9202, &first);
    let old = get_as::<IosSurface>(9202).unwrap();
    register(9202, &second);
    assert!(!old.is_alive());
    assert_eq!(first.released.load(Ordering::SeqCst), 0);
    drop(old);
    assert_eq!(first.released.load(Ordering::SeqCst), 1);
    destroy(9202);
    assert_eq!(second.released.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn invalid_pixels_do_not_enter_the_host_callback() {
    let recorder = Arc::new(Recorder::default());
    register(9203, &recorder);
    assert!(!draw_rgba(9203, 2, 2, &[0; 4]));
    assert!(!draw_rgba(9203, 0, 1, &[]));
    assert!(recorder.frames.lock().unwrap().is_empty());
    destroy(9203);
  }

  #[test]
  fn invalid_registration_leaves_ownership_with_the_caller() {
    let recorder = Arc::new(Recorder::default());
    let context = Arc::into_raw(recorder.clone()).cast_mut().cast();
    assert!(!unsafe {
      vessel_ios_surface_available(9204, std::ptr::null_mut(), context, Some(draw), Some(release), 2, 2)
    });
    assert!(!crate::is_alive(9204));
    assert_eq!(recorder.released.load(Ordering::SeqCst), 0);
    unsafe { release(context) };
    assert_eq!(recorder.released.load(Ordering::SeqCst), 1);
  }
}
