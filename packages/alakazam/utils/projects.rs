use std::rc::Rc;

use abra::Image;
use slint::ComponentHandle;
use slint::Global;
use slint::ModelRc;
use slint::SharedString;
use slint::VecModel;
use slint::Weak;

use crate::project::Project;
use crate::utils::image::as_ui_image;
use crate::utils::path::base_name;
use crate::AppState;
use crate::AppWindow;
use crate::UILayer;
use crate::UIProject;

use super::image::update_render;

/// Set the projects in the UI
/// This function will set the projects in the UI and update the UI
pub fn set_ui_projects(projects: &mut Vec<Project>, p_app: Weak<AppWindow>) {
  let mut v: Vec<UIProject> = vec![];
  for (i, p) in projects.iter().enumerate() {
    let base_name = base_name(&p.original_path);

    let mut layers = vec![];
    for layer in &p.layers {
      let image = as_ui_image(&layer.image);
      let layer = UILayer {
        name: SharedString::from(format!("{}", layer.name).to_string()),
        visible: true,
        opacity: 1.0,
        blend_mode: SharedString::from("normal".to_string()),
        image,
        focused: false,
      };
      layers.push(layer);
    }

    let project = UIProject {
      name: SharedString::from(format!("{}", base_name).to_string()),
      index: i as i32,
      dirty: false,
      zoom: 1.0,
      image_height: p.height as f32,
      image_width: p.width as f32,
      layers: ModelRc::from(Rc::new(VecModel::from(layers))),
    };
    v.push(project);
  }
  let new_projects = ModelRc::from(Rc::new(VecModel::from(v)));
  let app = p_app.upgrade().unwrap();
  let app_state = AppState::get(&app);
  app_state.set_projects(new_projects);
}
/// Add a new project to the UI
/// This function will add a new project to the UI and update the UI
pub fn add_project(projects: &mut Vec<Project>, image: Project, p_app: Weak<AppWindow>) {
  projects.push(image);
  set_ui_projects(projects, p_app);
}
/// Add multiple projects to the UI
/// This function will add multiple projects to the UI and update the UI
pub fn add_projects(projects: &mut Vec<Project>, new_projects: Vec<Project>, p_app: Weak<AppWindow>) {
  projects.extend(new_projects);
  set_ui_projects(projects, p_app);
}
/// Remove a project from the UI
/// This function will remove a project from the UI and update the UI
pub fn remove_project(projects: &mut Vec<Project>, index: usize, app: Weak<AppWindow>) {
  projects.remove(index);
  set_ui_projects(projects, app);
}

pub fn add_layer_to_project(projects: &mut Vec<Project>, index: usize, image: Image, p_app: Weak<AppWindow>) {
  let project = &mut projects[index];
  project.add_layer(image);
  let app = p_app.upgrade().unwrap();
  update_render(project, app.as_weak());
  set_ui_projects(projects, app.as_weak());
}
