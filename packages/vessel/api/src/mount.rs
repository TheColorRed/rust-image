//! Putting a component on a view that a host application owns, with nothing else for the app to do.
//!
//! The app names the view and gives it a component: [`mount`]. The host's view reports its surface whenever it has one,
//! and from then on the engine draws the component on it. The app never opens a session, attaches a surface, waits for the
//! view to appear or retries: the view may come before the component or after it, and may lose and regain its surface.

use std::collections::HashMap;
use std::sync::{Mutex, Once, OnceLock};

use pub_sub::{Observable, Observer};
use vessel_engine::surface::{self, SurfaceEvent, Touch};

use crate::{Component, PointerButton, PointerEvent, Renderable, Session};

/// A component put on a view, and the session drawing it while the view has a surface.
struct Mounted {
  component: Component,
  session: Option<Session>,
}

fn mounted() -> &'static Mutex<HashMap<i32, Mounted>> {
  static MOUNTED: OnceLock<Mutex<HashMap<i32, Mounted>>> = OnceLock::new();
  MOUNTED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Starts drawing the component mounted on `p_view` on the view's surface.
fn show(p_view: i32) {
  let Some(component) = mounted().lock().unwrap().get(&p_view).map(|mounted| mounted.component.clone()) else {
    return;
  };
  // The view's size is the component's size, whatever it was given before: that is what is on the screen.
  if let Some((width, height)) = surface::get(p_view).map(|surface| surface.size()).filter(|(width, height)| *width > 0 && *height > 0) {
    component.size().next((width, height));
  }
  // Opened outside the lock: the session starts an engine and draws, and neither may wait for the table.
  let session = Session::open(&component);
  session.attach_surface(p_view);
  let mut table = mounted().lock().unwrap();
  match table.get_mut(&p_view) {
    Some(mounted) if mounted.component.is(&component) => mounted.session = Some(session),
    // Replaced or unmounted meanwhile: this session is not wanted.
    _ => session.close(),
  }
}

/// Stops drawing on `p_view`'s surface, because it is gone.
fn hide(p_view: i32) {
  let session = mounted().lock().unwrap().get_mut(&p_view).and_then(|mounted| mounted.session.take());
  if let Some(session) = session {
    session.close();
  }
}

/// Gives the component mounted on `p_view` the view's new size, so it draws again to fit it. Without this a frame made for
/// the old size would be stretched over the new one by the host, which is what turning the screen would otherwise do.
fn resized(p_view: i32, p_width: u32, p_height: u32) {
  let Some(component) = mounted().lock().unwrap().get(&p_view).map(|mounted| mounted.component.clone()) else {
    return;
  };
  component.size().next((p_width, p_height));
}

/// Hands a touch on `p_view` to the component mounted there as the pointer events a mouse would send: it moves to the
/// place, and a button goes down or up. The view's pixels are the component's pixels, so the position needs no change.
fn touched(p_view: i32, p_touch: Touch, p_x: f32, p_y: f32) {
  let Some(component) = mounted().lock().unwrap().get(&p_view).map(|mounted| mounted.component.clone()) else {
    return;
  };
  component.next(PointerEvent::Moved { x: p_x, y: p_y });
  match p_touch {
    Touch::Moved => {}
    Touch::Down | Touch::Up => component.next(PointerEvent::Button {
      button: PointerButton::Left,
      pressed: p_touch == Touch::Down,
    }),
  }
}

/// Puts `p_component` on the view `p_view`, replacing whatever was mounted there, and draws it as soon as the view has a
/// surface. The view is whatever the host registered its surface under: for React Native, the `view` the app gives a
/// `VesselView`.
pub fn mount(p_view: i32, p_renderable: &impl Renderable) {
  let p_component = p_renderable.component();
  static LISTENING: Once = Once::new();
  LISTENING.call_once(|| {
    // The stream is persistent: the listener stays for good, so nothing has to hold the subscription.
    surface::events().subscribe(|event| match event {
      SurfaceEvent::Available(view) => show(*view),
      SurfaceEvent::Destroyed(view) => hide(*view),
      SurfaceEvent::Touched { id, touch, x, y } => touched(*id, *touch, *x, *y),
      SurfaceEvent::Resized { id, width, height } => resized(*id, *width, *height),
    });
  });

  let old = mounted().lock().unwrap().insert(
    p_view,
    Mounted {
      component: p_component.clone(),
      session: None,
    },
  );
  if let Some(session) = old.and_then(|old| old.session) {
    session.close();
  }
  // The view may have its surface already.
  if surface::is_alive(p_view) {
    show(p_view);
  }
}

/// Takes `p_component` off the view `p_view`, if it is the one mounted there, and stops drawing it.
pub fn unmount(p_view: i32, p_renderable: &impl Renderable) {
  let p_component = p_renderable.component();
  let mut table = mounted().lock().unwrap();
  if table.get(&p_view).is_some_and(|mounted| mounted.component.is(p_component)) {
    let removed = table.remove(&p_view);
    drop(table);
    if let Some(session) = removed.and_then(|removed| removed.session) {
      session.close();
    }
  }
}

/// Takes `p_component` off every view it is mounted on, for when it is going away.
pub fn unmount_all(p_renderable: &impl Renderable) {
  let p_component = p_renderable.component();
  let mut table = mounted().lock().unwrap();
  let views: Vec<i32> = table.iter().filter(|(_, mounted)| mounted.component.is(p_component)).map(|(view, _)| *view).collect();
  let removed: Vec<Mounted> = views.iter().filter_map(|view| table.remove(view)).collect();
  drop(table);
  for session in removed.into_iter().filter_map(|removed| removed.session) {
    session.close();
  }
}

#[cfg(test)]
mod tests {
  use std::any::Any;
  use std::sync::Arc;
  use std::time::{Duration, Instant};

  use super::*;
  use crate::Canvas;
  use pub_sub::prelude::*;

  struct Recorder(Mutex<Vec<u8>>);

  impl surface::Surface for Recorder {
    fn is_alive(&self) -> bool {
      true
    }

    fn size(&self) -> (u32, u32) {
      (2, 2)
    }

    fn resize(&self, _width: u32, _height: u32) {}

    fn draw_rgba(&self, _width: u32, _height: u32, p_rgba: &[u8]) -> bool {
      self.0.lock().unwrap().push(p_rgba[0]);
      true
    }

    fn retire(&self) {}

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
      self
    }
  }

  fn swatch(p_red: u8) -> Component {
    let component = Component::new("swatch").with_size(2, 2);
    component.subject::<Canvas>().subscribe(move |canvas| canvas.fill([p_red, 0, 0, 255]));
    component
  }

  fn wait_for(p_surface: &Recorder, p_red: u8) {
    let start = Instant::now();
    while p_surface.0.lock().unwrap().last() != Some(&p_red) {
      assert!(start.elapsed() < Duration::from_secs(5), "{p_red} was never drawn");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  #[test]
  fn a_component_is_drawn_when_its_view_gets_a_surface_whichever_came_first() {
    // The component first, then the surface.
    let first = Arc::new(Recorder(Mutex::new(Vec::new())));
    mount(8101, &swatch(11));
    surface::register_surface(8101, first.clone());
    wait_for(&first, 11);

    // The surface first, then the component.
    let second = Arc::new(Recorder(Mutex::new(Vec::new())));
    surface::register_surface(8102, second.clone());
    mount(8102, &swatch(22));
    wait_for(&second, 22);
  }

  #[test]
  fn mounting_again_replaces_the_component_and_unmounting_stops_it() {
    let view = Arc::new(Recorder(Mutex::new(Vec::new())));
    surface::register_surface(8103, view.clone());
    let old = swatch(33);
    mount(8103, &old);
    wait_for(&view, 33);

    let new = swatch(44);
    mount(8103, &new);
    wait_for(&view, 44);

    // An unmount of the component that was replaced leaves the new one alone.
    unmount(8103, &old);
    assert!(mounted().lock().unwrap().contains_key(&8103));
    unmount(8103, &new);
    assert!(!mounted().lock().unwrap().contains_key(&8103));
  }

  #[test]
  fn a_view_that_changes_size_gives_the_component_the_new_size_to_draw_at() {
    let view = Arc::new(Recorder(Mutex::new(Vec::new())));
    surface::register_surface(8104, view.clone());
    let component = swatch(5);
    mount(8104, &component);
    assert_eq!(component.size().value(), (2, 2), "it takes the view's size when it is shown");

    // The screen turns: the view is wider and shorter.
    surface::resize(8104, 6, 1);
    assert_eq!(component.size().value(), (6, 1));
  }
}
