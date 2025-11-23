use crate::{AppState, AppWindow};
use slint::{ComponentHandle, Global, Weak};
use std::sync::Arc;

pub fn on_request_minimize(p_app: Weak<AppWindow>) {
  let app = p_app.upgrade().unwrap();
  let app_state = AppState::get(&app);
  app_state.on_request_minimize(move || {
    let app = p_app.upgrade().unwrap();
    app.window().set_minimized(true);
  });
}

pub fn on_request_maximize(p_app: Weak<AppWindow>) {
  let app = p_app.upgrade().unwrap();
  let app_state = AppState::get(&app);
  app_state.on_request_maximize(move || {
    let app = p_app.upgrade().unwrap();
    app.window().set_maximized(true);
  });
}

pub fn on_request_restore(p_app: Weak<AppWindow>) {
  let app = p_app.upgrade().unwrap();
  let app_state = AppState::get(&app);
  app_state.on_request_restore(move || {
    let app = p_app.upgrade().unwrap();
    app.window().set_maximized(false);
  });
}

pub fn on_request_quit(p_app: Weak<AppWindow>) {
  let app = p_app.upgrade().unwrap();
  let app_state = AppState::get(&app);
  app_state.on_request_close(move || {
    let app = p_app.upgrade().unwrap();
    app.hide().expect("Failed to close the app");
  });
}

pub fn on_request_size_toggle(p_app: Weak<AppWindow>) {
  let app = p_app.upgrade().unwrap();
  let app_state = AppState::get(&app);
  app_state.on_request_size_toggle(move || {
    let app = p_app.upgrade().unwrap();
    let is_maximized = app.window().is_maximized();
    app.window().set_maximized(!is_maximized);
  });
}
