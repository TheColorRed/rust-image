//! The native methods of `dev.vessel.android.SurfaceBridge`: a view in the host app reports its surface here.
//!
//! Any Android host UI (React Native, Flutter, a Java or Kotlin app) can use the Java class, so the surface is registered
//! under the id the host gives it and the rest of Vessel finds it there. The names are fixed by the Java package.

use std::ffi::c_void;

/// A view's surface became available.
///
/// # Safety
/// Called by the JVM, with the calling thread's `JNIEnv` and a `Surface` reference from it.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_dev_vessel_android_SurfaceBridge_nativeSurfaceAvailable(
  p_env: *mut c_void, _p_class: *mut c_void, p_id: i32, p_surface: *mut c_void, p_width: i32, p_height: i32,
) {
  unsafe { crate::register_android_surface(p_id, p_env, p_surface, p_width.max(1) as u32, p_height.max(1) as u32) };
}

/// A view's surface changed size.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_vessel_android_SurfaceBridge_nativeSurfaceSizeChanged(
  _p_env: *mut c_void, _p_class: *mut c_void, p_id: i32, p_width: i32, p_height: i32,
) {
  crate::resize(p_id, p_width.max(1) as u32, p_height.max(1) as u32);
}

/// A view was touched: `p_action` is 0 for down, 1 for moved and 2 for up or cancelled.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_vessel_android_SurfaceBridge_nativeTouch(
  _p_env: *mut c_void, _p_class: *mut c_void, p_id: i32, p_action: i32, p_x: f32, p_y: f32,
) {
  let touch = match p_action {
    0 => crate::Touch::Down,
    1 => crate::Touch::Moved,
    _ => crate::Touch::Up,
  };
  crate::touch(p_id, touch, p_x, p_y);
}

/// A view's surface is gone.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_vessel_android_SurfaceBridge_nativeSurfaceDestroyed(
  _p_env: *mut c_void, _p_class: *mut c_void, p_id: i32,
) {
  crate::destroy(p_id);
}
