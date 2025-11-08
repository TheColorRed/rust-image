use std::sync::{Arc, Mutex};

use crate::events::{dispatch, dispatch_to, EventSystem, EventSystemType};
use crate::AppWindow;
use i_slint_backend_winit::WinitWindowAccessor;
use i_slint_core::component_factory::ComponentFactory;
use slint::{ComponentHandle, Global, PhysicalSize, SharedString, Weak, WindowSize};
use winit::platform::windows::WindowExtWindows;
use winit::window::Window;

mod dialog_window {
  slint::slint! {
    export { DialogWindow } from "ui/dialog-window.slint";
    export { NewProjectDialog } from "ui/dialogs/new-project/main.slint";
    export { DialogState } from "ui/globals/dialog-state.slint";
  }
}

pub struct BaseDialog {
  pub parent: Weak<AppWindow>,
  pub dialog: Weak<dialog_window::DialogWindow>,
  pub events: EventSystemType,
}

impl Clone for BaseDialog {
  fn clone(&self) -> Self {
    BaseDialog {
      parent: self.parent.clone(),
      dialog: self.dialog.clone(),
      events: self.events.clone(),
    }
  }
}

enum WindowType {
  Main(Weak<AppWindow>),
  Dialog(Weak<dialog_window::DialogWindow>),
}

impl BaseDialog {
  pub fn open(parent: Weak<AppWindow>, app_type: &str, title: &str, mut size: (u32, u32)) -> Self {
    // let factory = ComponentFactory::new(|ctx| {});

    let dialog = dialog_window::DialogWindow::new().unwrap();
    let weak_dialog = dialog.as_weak();
    let base_dialog_window = BaseDialog {
      parent: parent.clone(),
      dialog: weak_dialog.clone(),
      events: Arc::new(Mutex::new(EventSystem::new())),
    };

    dialog.set_window_type(SharedString::from(app_type));
    dialog.set_window_title(SharedString::from(title));

    let source_winit_window = base_dialog_window.get_winit_window(WindowType::Main(parent.clone()));
    let dialog_winit_window = base_dialog_window.get_winit_window(WindowType::Dialog(weak_dialog.clone()));

    dialog_winit_window.unwrap().set_resizable(false);
    dialog_winit_window.unwrap().set_skip_taskbar(true);
    dialog_winit_window.unwrap().set_undecorated_shadow(true);
    source_winit_window.unwrap().set_enable(false);

    let clone = weak_dialog.clone().unwrap();
    let dialog_state = dialog_window::DialogState::get(&clone);

    let app_weak = weak_dialog.clone();
    dialog_state.on_window_move(move || {
      let app = app_weak.unwrap();
      app.window().with_winit_window(|winit| winit.drag_window());
    });

    let app_weak = weak_dialog.clone();
    let src = base_dialog_window.clone();
    dialog_state.on_close(move || {
      let app = app_weak.unwrap();
      let base = src.get_winit_window(WindowType::Main(parent.clone()));
      base.unwrap().set_enable(true);
      app.hide().expect("Unable to close dialog window");
    });

    dialog_state.on_dispatch_global_event(move |event| {
      dispatch(event.as_str(), None);
    });

    // TODO: Implement dispatching to parent
    let src = base_dialog_window.clone();
    dialog_state.on_dispatch_parent_event(move |event| {
      dispatch_to(&src.events, event.as_str(), None);
    });

    let src = base_dialog_window.clone();
    dialog_state.on_dispatch_self_event(move |event| {
      dispatch_to(&src.events, event.as_str(), None);
    });

    if let Err(e) = base_dialog_window.dialog.unwrap().show() {
      eprintln!("Failed to show window: {:?}", e);
    }

    size.1 = size.1 + 30;
    dialog.window().set_size(WindowSize::Physical(PhysicalSize::new(size.0, size.1)));

    base_dialog_window
  }

  fn get_winit_window(&self, window: WindowType) -> Option<&Window> {
    let found_window = match window {
      WindowType::Main(app) => app.unwrap().window().with_winit_window(move |winit| {
        let win: &'static Window = unsafe { std::mem::transmute(winit) };
        win
      }),
      WindowType::Dialog(app) => app.unwrap().window().with_winit_window(move |winit| {
        let win: &'static Window = unsafe { std::mem::transmute(winit) };
        win
      }),
    };

    found_window
  }
}
