use crate::project::Project;
use bincode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use rfd::FileDialog;

#[derive(Serialize, Deserialize)]
pub struct ImageDataSaved {
  pub width: u32,
  pub height: u32,
  pub colors: Vec<u8>,
}

pub fn save_file_dialog(extensions: Vec<&str>) -> Option<PathBuf> {
  FileDialog::new().add_filter("Save File", &extensions).save_file()
}

pub fn save_file(project: &Project) {
  let path = save_file_dialog(vec!["akz"]);
  let colors = project.image.rgba();
  match path {
    Some(p) => {
      let image_data = ImageDataSaved {
        width: project.width,
        height: project.height,
        colors,
      };
      let image_data = bincode::serialize(&image_data).unwrap();
      std::fs::write(p, image_data).unwrap();
    }
    None => (),
  };
}
