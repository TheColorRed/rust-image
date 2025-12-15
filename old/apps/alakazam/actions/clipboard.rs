use std::sync::{Arc, Mutex};

use abra::Image;
use arboard::Clipboard;
use slint::{ComponentHandle, Global, Weak};

use crate::{
  utils::projects::{add_layer_to_project, add_projects},
  AppProject, AppState, AppWindow,
};

pub fn on_paste(p_app: Weak<AppWindow>, p_projects: &Arc<Mutex<Vec<AppProject>>>) {
  let app = p_app.upgrade().unwrap();
  let projects = p_projects.clone();
  let app_state = AppState::get(&app);
  app_state.on_paste(move || {
    let mut clipboard = Clipboard::new().unwrap();
    let app = p_app.upgrade().unwrap();
    let app_state = AppState::get(&app);
    let img = clipboard.get_image().expect("Failed to get image from clipboard");
    let mut image = Image::new(img.width as u32, img.height as u32);
    let active_project = app_state.get_active_project() as i32;
    image.set_rgba_owned(img.bytes.to_vec());
    let app = p_app.upgrade().unwrap();
    if active_project > -1 {
      let mut projects_mut = projects.lock().unwrap();
      add_layer_to_project(&mut projects_mut, active_project as usize, image, app.as_weak());
    } else {
      let project = AppProject::new_from_image(image);
      add_projects(&mut projects.lock().unwrap(), vec![project], app.as_weak());
      app_state.set_active_project(0);
    }
  });
}
