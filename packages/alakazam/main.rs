use abra::Image;
use actions::{clipboard, tabs::Tabs, trigger, window};
use components::color_picker;
use slint::Model;
use std::sync::{Arc, Mutex};
use winit::{platform::windows::WindowExtWindows, window::CustomCursor};

use i_slint_backend_winit::WinitWindowAccessor;

mod actions;
mod dialogs;
mod events;
mod project;
mod utils;

pub mod components;

slint::slint! {
  export { AppWindow, AppMenu, LayerActions} from "ui/main.slint";
  export { AppState, UIProject } from "ui/globals/app-state.slint";
  export { ColorPanel, ColorType } from "ui/components/panels/colors/colors.slint";
}

pub struct AbraProject(pub abra::canvas::Canvas);
use crate::project::Project as AppProject;

fn main() -> Result<(), slint::PlatformError> {
  let app = AppWindow::new().unwrap();
  let app_window = app.as_weak();
  let projects_original: Arc<Mutex<Vec<AppProject>>> = Arc::new(Mutex::new(vec![]));

  // let cursor_image = Image::new_from_path("packages/alakazam/ui/icons/house.svg");
  // let cursor = CustomCursor::from_rgba(cursor_image.rgba(), 16, 16, 0, 0).unwrap();
  let app = app_window.clone().unwrap();
  app.window().with_winit_window(|winit| {
    winit.set_undecorated_shadow(true);
    // winit.set_cursor(cursor.into());
  });

  // Initialize windows
  crate::dialogs::init(app_window.clone());

  // Tabs actions
  Tabs::init(app_window.clone(), &projects_original);

  // Trigger actions
  trigger::on_trigger_action(app_window.clone(), &projects_original);

  // Clipboard actions
  clipboard::on_paste(app_window.clone(), &projects_original);

  // Window sizing actions
  window::on_request_minimize(app_window.clone());
  window::on_request_maximize(app_window.clone());
  window::on_request_restore(app_window.clone());
  window::on_request_quit(app_window.clone());
  window::on_request_size_toggle(app_window.clone());

  // Add Components
  color_picker::entry(app_window.clone());

  let weak_app = app_window.clone();
  let temp_app = weak_app.unwrap();
  let layer_actions = LayerActions::get(&temp_app);
  layer_actions.on_selected_layer_changed(move |selected_layers| {
    let app = weak_app.upgrade().unwrap();
    let projects = AppState::get(&app).get_projects();
    for p in projects.iter() {
      for mut item in p.layers.iter() {
        let is_changed = selected_layers.iter().any(|l| l == item);
        item.focused = is_changed;
      }
    }
    AppState::get(&app).set_projects(projects);

    app.window().request_redraw();
  });

  let weak_app = app_window.clone();
  slint::invoke_from_event_loop(move || {
    weak_app.unwrap().window().set_maximized(true);
  })
  .unwrap();

  let weak_app = app_window.clone();
  let temp_app = app_window.clone();
  AppState::get(&temp_app.unwrap()).on_window_move(move || {
    let app_clone = weak_app.unwrap();
    app_clone.window().with_winit_window(|winit| winit.drag_window());
  });

  app.run()?;
  Ok(())
}
