use super::base_dialog_window::BaseDialog;
use crate::dialogs::color_picker::show_color_picker_window;
use crate::events::event_list::OPEN_COLOR_PICKER_WINDOW;
use crate::events::{add_listener, event_list::OPEN_NEW_FILE_WINDOW};
use crate::events::{add_listener_to, EventSystemType};
use crate::AppWindow;
use slint::Weak;

pub fn init(parent: Weak<AppWindow>) {
  add_listener(OPEN_NEW_FILE_WINDOW, move |_| {
    let new_project = BaseDialog::open(parent.clone(), "new-project", "New Project", (1000, 700));
    listen_for_color_picker(parent.clone(), &new_project.events);
  });
}

fn listen_for_color_picker(parent: Weak<AppWindow>, events: &EventSystemType) {
  add_listener_to(events, OPEN_COLOR_PICKER_WINDOW, move |_| {
    show_color_picker_window(parent.clone());
  });
}
