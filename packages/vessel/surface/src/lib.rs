//! Surfaces that frames are drawn on, looked up by the id the platform's view was given.
//!
//! A [`Surface`] is anything frames can be drawn on: a native window today, and later a desktop window, a mobile view
//! layer, a web canvas or a texture. A renderer such as a live view engine knows only the trait and the id, so a new
//! kind of surface is added by implementing [`Surface`] and registering it; the renderer and the apps do not change.
//!
//! The platform's view registers its surface here when it appears, resizes it and destroys it when it goes away, so
//! frames never have to pass through the app's scripting layer.
//!
//! Each platform is a cargo feature, off by default, named for how the surface is owned:
//!
//! - `*-surface`: inline component style, drawing inside a view that a host application owns.
//! - `*-window`: a standalone app or game, where Vessel owns the window.
//!
//! A feature on a platform the build does not target compiles to nothing.
//!
//! | Feature | Surface kind |
//! |---|---|
//! | `android-surface` | `AndroidWindow`: an Android `Surface` handed over by any host UI |
//! | `ios-surface` | `IosSurface`: a retained Metal layer handed over by any host UI |
//! | `desktop-window` | A native desktop window that Vessel opens and runs (`desktop::run`) |
#![deny(missing_docs)]

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use pub_sub::{Observer, Subject};

#[cfg(feature = "android-native-window")]
mod android;
#[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
pub mod desktop;
#[cfg(feature = "ios-surface")]
mod ios;
#[cfg(feature = "android-native-window")]
mod jni;

#[cfg(all(feature = "android-native-window", target_os = "android"))]
pub use android::register_android_surface;
#[cfg(feature = "android-native-window")]
pub use android::{AndroidWindow, AndroidWindowState, blit_rgba, register_android_window};
#[cfg(feature = "ios-surface")]
pub use ios::{
  IosSurface, vessel_ios_surface_available, vessel_ios_surface_destroyed, vessel_ios_surface_resized, vessel_ios_touch,
};

/// Something frames can be drawn on.
pub trait Surface: Any + Send + Sync {
  /// Whether the platform still has the surface. Nothing may be drawn on it after it is gone.
  fn is_alive(&self) -> bool;

  /// The surface's size in pixels.
  fn size(&self) -> (u32, u32);

  /// Records a new size after the platform resized the surface.
  fn resize(&self, p_width: u32, p_height: u32);

  /// Draws `p_rgba` (`p_width * p_height` RGBA pixels), scaled to the surface, and returns whether the surface took it.
  /// A surface that is gone returns `false`.
  fn draw_rgba(&self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> bool;

  /// Marks the surface gone. Called when the platform destroys it or another surface takes its id. It waits for a frame
  /// being drawn, so the platform can free the surface afterwards.
  fn retire(&self);

  /// This surface as `Any`, so code that knows the kind it registered can get it back with [`get_as`].
  fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

fn registry() -> &'static Mutex<HashMap<i32, Arc<dyn Surface>>> {
  static REGISTRY: OnceLock<Mutex<HashMap<i32, Arc<dyn Surface>>>> = OnceLock::new();
  REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The surface currently registered under `p_id`.
pub fn get(p_id: i32) -> Option<Arc<dyn Surface>> {
  registry().lock().ok()?.get(&p_id).cloned()
}

/// The surface registered under `p_id` if it is a `T`, for code that needs the kind it registered, such as its native
/// handle.
pub fn get_as<T: Surface>(p_id: i32) -> Option<Arc<T>> {
  get(p_id)?.into_any().downcast::<T>().ok()
}

/// How a finger (or any pointer) touched a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Touch {
  /// It went down.
  Down,
  /// It moved.
  Moved,
  /// It came up, or the touch was cancelled.
  Up,
}

/// What happened to a surface a host registered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceEvent {
  /// A surface is now registered under this id.
  Available(i32),
  /// The surface registered under this id is gone.
  Destroyed(i32),
  /// The surface registered under this id changed size, for example because the screen turned.
  Resized {
    /// The surface's id.
    id: i32,
    /// Its new width in pixels.
    width: u32,
    /// Its new height in pixels.
    height: u32,
  },
  /// The surface registered under this id was touched, at `x` and `y` pixels from its top left corner.
  Touched {
    /// The surface's id.
    id: i32,
    /// How it was touched.
    touch: Touch,
    /// Distance from the left edge.
    x: f32,
    /// Distance from the top edge.
    y: f32,
  },
}

/// Tells everything listening to [`events`] that the surface registered under `p_id` was touched.
pub fn touch(p_id: i32, p_touch: Touch, p_x: f32, p_y: f32) {
  events().next(SurfaceEvent::Touched {
    id: p_id,
    touch: p_touch,
    x: p_x,
    y: p_y,
  });
}

/// Every surface that appears or goes away, as a pub-sub stream, so whatever shows things on surfaces (the `Session`s of
/// `vessel-api`) can start when a host's view gets its surface, without the app listening or retrying.
pub fn events() -> Subject<SurfaceEvent> {
  static EVENTS: OnceLock<Subject<SurfaceEvent>> = OnceLock::new();
  EVENTS.get_or_init(Subject::persistent).clone()
}

/// Registers `p_surface` under `p_id`, retiring any earlier surface under the same id.
pub fn register_surface(p_id: i32, p_surface: Arc<dyn Surface>) {
  // The registry lock is released before the surface's own lock is taken: drawing takes them in the other order.
  let old = registry().lock().unwrap().insert(p_id, p_surface);
  if let Some(old) = old {
    old.retire();
  }
  events().next(SurfaceEvent::Available(p_id));
}

/// Forgets the surface registered under `p_id` and retires it.
pub fn destroy(p_id: i32) {
  let removed = registry().lock().unwrap().remove(&p_id);
  if let Some(surface) = removed {
    surface.retire();
    events().next(SurfaceEvent::Destroyed(p_id));
  }
}

/// Records a new size for the surface registered under `p_id`, if any.
pub fn resize(p_id: i32, p_width: u32, p_height: u32) {
  if let Some(surface) = get(p_id) {
    surface.resize(p_width, p_height);
    events().next(SurfaceEvent::Resized {
      id: p_id,
      width: p_width,
      height: p_height,
    });
  }
}

/// Whether a surface is registered under `p_id` and still alive.
pub fn is_alive(p_id: i32) -> bool {
  get(p_id).is_some_and(|surface| surface.is_alive())
}

/// Draws RGBA pixels on the surface registered under `p_id`, and returns whether there was a live one that took them.
/// - `p_rgba`: `p_width * p_height` RGBA pixels.
pub fn draw_rgba(p_id: i32, p_width: u32, p_height: u32, p_rgba: &[u8]) -> bool {
  get(p_id).is_some_and(|surface| surface.draw_rgba(p_width, p_height, p_rgba))
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

  use super::*;

  // The registry is shared by every test, so each test uses ids of its own.

  /// A surface kind of the simplest sort, to show the registry does not care what it holds.
  #[derive(Default)]
  struct Counting {
    drawn: AtomicUsize,
    gone: AtomicBool,
  }

  impl Surface for Counting {
    fn is_alive(&self) -> bool {
      !self.gone.load(Ordering::SeqCst)
    }

    fn size(&self) -> (u32, u32) {
      (1, 1)
    }

    fn resize(&self, _p_width: u32, _p_height: u32) {}

    fn draw_rgba(&self, _p_width: u32, _p_height: u32, _p_rgba: &[u8]) -> bool {
      if !self.is_alive() {
        return false;
      }
      self.drawn.fetch_add(1, Ordering::SeqCst);
      true
    }

    fn retire(&self) {
      self.gone.store(true, Ordering::SeqCst);
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
      self
    }
  }

  #[test]
  fn any_kind_of_surface_can_be_registered_and_drawn_on() {
    let counting = Arc::new(Counting::default());
    register_surface(9_101, counting.clone());
    assert!(draw_rgba(9_101, 1, 1, &[0; 4]));
    assert!(draw_rgba(9_101, 1, 1, &[0; 4]));
    assert_eq!(counting.drawn.load(Ordering::SeqCst), 2);
    destroy(9_101);
    assert!(!counting.is_alive());
    assert!(!draw_rgba(9_101, 1, 1, &[0; 4]));
  }

  #[test]
  fn registering_again_under_the_same_id_retires_the_old_surface() {
    let (first, second) = (Arc::new(Counting::default()), Arc::new(Counting::default()));
    register_surface(9_102, first.clone());
    register_surface(9_102, second.clone());
    assert!(!first.is_alive());
    assert!(second.is_alive());
    destroy(9_102);
  }

  #[test]
  fn destroying_removes_the_surface_and_retires_it() {
    let surface = Arc::new(Counting::default());
    register_surface(9_103, surface.clone());
    destroy(9_103);
    assert!(get(9_103).is_none());
    assert!(!surface.is_alive());
    assert!(!is_alive(9_103));
  }

  #[test]
  fn a_surface_can_be_gotten_back_as_its_own_kind() {
    register_surface(9_104, Arc::new(Counting::default()));
    assert!(get_as::<Counting>(9_104).is_some());
    destroy(9_104);
    assert!(get_as::<Counting>(9_104).is_none());
  }

  #[test]
  fn resizing_reaches_the_surface() {
    struct Sized(Mutex<(u32, u32)>);
    impl Surface for Sized {
      fn is_alive(&self) -> bool {
        true
      }
      fn size(&self) -> (u32, u32) {
        *self.0.lock().unwrap()
      }
      fn resize(&self, p_width: u32, p_height: u32) {
        *self.0.lock().unwrap() = (p_width, p_height);
      }
      fn draw_rgba(&self, _: u32, _: u32, _: &[u8]) -> bool {
        true
      }
      fn retire(&self) {}
      fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
      }
    }
    let surface = Arc::new(Sized(Mutex::new((1, 1))));
    register_surface(9_105, surface.clone());
    resize(9_105, 30, 40);
    assert_eq!(surface.size(), (30, 40));
    destroy(9_105);
  }
}
