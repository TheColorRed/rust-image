//! Native window surfaces handed over by the platform's preview view, looked up by the id the view was given.
//!
//! The view (an Android `TextureView`) reports its surface here when it appears, resizes or goes away. A live preview
//! attached to that id draws on the window directly, so its frames never pass through JavaScript.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, OnceLock};

/// What a preview needs to know about a window while drawing on it.
pub struct WindowState {
  /// False once the platform destroyed the surface; nothing may be drawn after that.
  pub alive: bool,
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
}

/// A native window. Its lock is held while drawing, so the platform's destroy call waits for the frame in flight.
pub struct Window {
  ptr: *mut c_void,
  /// Changes with the window's state; also the lock that serializes drawing against destruction.
  pub state: Mutex<WindowState>,
}

// The pointer is an `ANativeWindow`, which is thread-safe and reference counted.
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
  /// The raw `ANativeWindow` pointer.
  pub fn ptr(&self) -> *mut c_void {
    self.ptr
  }
}

impl Drop for Window {
  fn drop(&mut self) {
    #[cfg(target_os = "android")]
    unsafe {
      ndk_sys::ANativeWindow_release(self.ptr.cast());
    }
  }
}

fn registry() -> &'static Mutex<HashMap<i32, Arc<Window>>> {
  static REGISTRY: OnceLock<Mutex<HashMap<i32, Arc<Window>>>> = OnceLock::new();
  REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The window currently registered under `p_id`.
pub fn get(p_id: i32) -> Option<Arc<Window>> {
  registry().lock().ok()?.get(&p_id).cloned()
}

/// Registers a window that owns one reference to `p_ptr`, replacing (and killing) any earlier one under the same id.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn register(p_id: i32, p_ptr: *mut c_void, p_width: u32, p_height: u32) {
  let window = Arc::new(Window {
    ptr: p_ptr,
    state: Mutex::new(WindowState {
      alive: true,
      width: p_width,
      height: p_height,
    }),
  });
  // The registry lock is released before the window's own lock is taken: drawing takes them in the other order.
  let old = registry().lock().unwrap().insert(p_id, window);
  if let Some(old) = old {
    old.state.lock().unwrap().alive = false;
  }
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn resize(p_id: i32, p_width: u32, p_height: u32) {
  if let Some(window) = get(p_id) {
    let mut state = window.state.lock().unwrap();
    state.width = p_width;
    state.height = p_height;
  }
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn destroy(p_id: i32) {
  let removed = registry().lock().unwrap().remove(&p_id);
  if let Some(window) = removed {
    window.state.lock().unwrap().alive = false;
  }
}

#[cfg(target_os = "android")]
mod jni {
  use super::{destroy, register, resize};
  use std::ffi::c_void;

  #[unsafe(no_mangle)]
  pub extern "system" fn Java_com_alakazam_mobile_abra_AbraLiveViewManager_nativeSurfaceAvailable(
    p_env: *mut c_void,
    _p_class: *mut c_void,
    p_id: i32,
    p_surface: *mut c_void,
    p_width: i32,
    p_height: i32,
  ) {
    let window = unsafe { ndk_sys::ANativeWindow_fromSurface(p_env.cast(), p_surface.cast()) };
    if !window.is_null() {
      register(p_id, window.cast(), p_width.max(1) as u32, p_height.max(1) as u32);
    }
  }

  #[unsafe(no_mangle)]
  pub extern "system" fn Java_com_alakazam_mobile_abra_AbraLiveViewManager_nativeSurfaceSizeChanged(
    _p_env: *mut c_void,
    _p_class: *mut c_void,
    p_id: i32,
    p_width: i32,
    p_height: i32,
  ) {
    resize(p_id, p_width.max(1) as u32, p_height.max(1) as u32);
  }

  #[unsafe(no_mangle)]
  pub extern "system" fn Java_com_alakazam_mobile_abra_AbraLiveViewManager_nativeSurfaceDestroyed(
    _p_env: *mut c_void,
    _p_class: *mut c_void,
    p_id: i32,
  ) {
    destroy(p_id);
  }
}
