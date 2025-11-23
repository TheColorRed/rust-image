use crate::project::Project;
use abra::{Channels, Image};
use rfd::FileDialog;

use super::save_file::ImageDataSaved;

pub struct ImageDataLoaded {
  pub path: String,
  pub image: Image,
}

pub fn open_file_dialog() -> Option<Vec<ImageDataLoaded>> {
  let files = FileDialog::new()
    .add_filter("Open Files", &["akz", "jpg", "jpeg", "png", "webp"])
    .pick_files();

  if let Some(paths) = files {
    let mut images = vec![];
    for path in paths {
      if path.to_str().unwrap().ends_with(".akz") {
        let full_path = path.to_str().unwrap();
        let bytes = std::fs::read(full_path).unwrap();
        let d: ImageDataSaved = bincode::deserialize(&bytes).unwrap();
        let img = {
            let width = d.width;
            let height = d.height;
            let pixels = d.colors;
            let channels = Channels::RGBA;
            let mut img = Image::new(width, height);
            match channels {
              Channels::RGBA => img.set_rgba(pixels),
              Channels::RGB => img.set_rgb(pixels),
            }
            // img.set_rgba(pixels);
            img
          };
        images.push(ImageDataLoaded {
          path: full_path.to_string(),
          image: img,
        });
      } else {
        let full_path = path.to_str().unwrap();
        let img = Image::new_from_path(full_path);
        images.push(ImageDataLoaded {
          path: full_path.to_string(),
          image: img,
        });
      }
    }
    return Some(images);
  }
  None
}

pub fn open_file() -> Option<Vec<Project>> {
  let img_dialog = open_file_dialog();
  match img_dialog {
    Some(img) => {
      let mut projects = vec![];
      for i in img {
        projects.push(Project::new_from_image_data(i));
      }
      Some(projects)
    }
    None => None,
  }
}
