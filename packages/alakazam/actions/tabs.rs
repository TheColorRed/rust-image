use crate::{
  utils::{display_image, projects::remove_project},
  AppProject, AppState, AppWindow,
};
use slint::{ComponentHandle, Global, Weak};
use std::sync::{Arc, Mutex};

pub struct Tabs;

impl Tabs {
  pub fn init(p_app: Weak<AppWindow>, p_projects: &Arc<Mutex<Vec<AppProject>>>) {
    Tabs::on_active_project_changed(p_app.clone(), p_projects);
    Tabs::on_close_tab(p_app.clone(), p_projects);
  }

  fn on_active_project_changed(p_app: Weak<AppWindow>, p_projects: &Arc<Mutex<Vec<AppProject>>>) {
    let projects = p_projects.clone();
    let app = p_app.upgrade().unwrap();
    let state = AppState::get(&app);
    state.on_active_project_changed(move |index| {
      let active_project = index as usize;
      let project = &projects.lock().unwrap()[active_project];
      display_image(project, &p_app);
    });
  }

  fn on_close_tab(p_app: Weak<AppWindow>, p_projects: &Arc<Mutex<Vec<AppProject>>>) {
    let projects = p_projects.clone();
    let app = p_app.upgrade().unwrap();
    let state = AppState::get(&app);
    state.on_close_tab(move |value| {
      let mut projects_mut = projects.lock().unwrap();
      let index = value as usize;
      let app = p_app.upgrade().unwrap();
      remove_project(&mut projects_mut, index, app.as_weak());
      if projects_mut.len() > 0 && projects_mut.len() > index {
        let last = projects_mut.len() - 1;
        display_image(&projects_mut[last as usize], &p_app);
      }
    });
  }
}
