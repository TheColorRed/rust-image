use slint::{ComponentHandle, Global, Weak};

use crate::actions::open_file::open_file;
use crate::actions::save_file::save_file;
use crate::events::dispatch;
use crate::events::event_list::OPEN_NEW_FILE_WINDOW;
use crate::utils::projects::add_projects;
use crate::AppWindow;
use crate::{AppMenu, AppProject, AppState};
use std::sync::{Arc, Mutex};

pub fn on_trigger_action(p_app: Weak<AppWindow>, p_projects: &Arc<Mutex<Vec<AppProject>>>) {
  let projects = p_projects.clone();
  let app = p_app.upgrade().unwrap();
  let temp_app = p_app.upgrade().unwrap();
  let app_menu = AppMenu::get(&temp_app);
  app_menu.on_trigger_action(move |value| {
    println!("Trigger Action {}", value);
    let mut projects_mut = projects.lock().unwrap();
    let app_state = AppState::get(&app);
    match value.as_str() {
      "exit" => app.hide().expect("Failed to close the app"),
      "open" => {
        if let Some(new_projects) = open_file() {
          let app = p_app.upgrade().unwrap();
          add_projects(&mut projects_mut, new_projects, app.as_weak());
          app_state.set_active_project(projects_mut.len() as i32 - 1);
        }
      }
      "save" => {
        let active = AppState::get(&app).get_active_project();
        let project = &projects_mut[active as usize];
        save_file(&project);
      }
      "new" => {
        dispatch(OPEN_NEW_FILE_WINDOW, None);
      }
      _ => {}
    }
  });
}
