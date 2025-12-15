use super::base_dialog_window::BaseDialog;
use crate::events::{add_listener, event_list::OPEN_COLOR_PICKER_WINDOW};
use crate::AppWindow;
use slint::Weak;

pub fn show_color_picker_window(base: Weak<AppWindow>) -> BaseDialog {
  let app = base.clone();
  let size = (500, 500);
  BaseDialog::open(app, "color-picker", "Color Picker", size)
}

pub fn init(base: Weak<AppWindow>) {
  add_listener(OPEN_COLOR_PICKER_WINDOW, move |_| {
    show_color_picker_window(base.clone());
  });
}
