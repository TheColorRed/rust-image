mod base_dialog_window;
mod color_picker;
mod new_file;

use crate::AppWindow;
use slint::Weak;

/// Initialize all GUI window APIs
pub fn init(app: Weak<AppWindow>) {
  new_file::init(app.clone());
  color_picker::init(app.clone());
}
