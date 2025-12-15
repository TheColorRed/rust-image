use crate::project::Project;
use crate::AppState;
use crate::AppWindow;
use abra::Image;
use slint::Global;
use slint::Weak;
use slint::{Rgba8Pixel, SharedPixelBuffer};

pub fn display_image(project: &Project, app: &Weak<AppWindow>) {
  let img = project.image.clone();
  let ui_image = as_ui_image(&img);
  AppState::get(&app.upgrade().unwrap()).set_stage_image(ui_image);
}

pub fn as_ui_image(image: &Image) -> slint::Image {
  let (width, height) = image.dimensions();
  let colors = image.rgba();
  let pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(colors.as_slice(), width, height);
  slint::Image::from_rgba8(pixel_buffer)
}

pub fn vec_as_ui_image(colors: Vec<u8>, width: u32, height: u32) -> slint::Image {
  let pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(colors.as_slice(), width, height);
  slint::Image::from_rgba8(pixel_buffer)
}

pub fn update_render(project: &mut Project, app: Weak<AppWindow>) {
  let mut image = project.image.clone();
  for layer in &project.layers {
    // image = blend_images_at(&image, &layer.image, 0, 0, 0, 0, layer.blend_mode.as_ref());
  }
  // project.image = image;
  // display_image(project, app);
}
