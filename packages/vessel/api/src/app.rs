//! Running an app that owns its window.
//!
//! [`run`] shows a [`Component`] in a window of the platform the build targets and returns when the app ends. The
//! component never knows which platform that is: the host sets its size, sends it input as events, and ends the app when
//! the component sends [`Quit`](crate::Quit).

use pub_sub::prelude::*;

use crate::{Background, Component, KeyEvent, Quit, Renderable};

/// How the app's window is framed. A host with no frame to speak of, such as a phone, ignores it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppFrame {
  /// A normal window with a title bar and a border.
  #[default]
  Bordered,
  /// A window with no title bar and no border.
  Borderless,
  /// A borderless window that covers the whole screen, the usual mode for a game.
  Fullscreen,
}

/// What the app's window should look like, in terms every platform understands. Build it with [`AppOptions::new`] and the
/// `with_*` methods.
#[derive(Clone, Debug, PartialEq)]
pub struct AppOptions {
  /// The title shown in the title bar and the task bar.
  pub title: String,
  /// The inner width in logical pixels.
  pub width: u32,
  /// The inner height in logical pixels.
  pub height: u32,
  /// Whether the user can resize the window.
  pub resizable: bool,
  /// How the window is framed.
  pub frame: AppFrame,
}

impl AppOptions {
  /// A bordered, resizable 1280x720 window.
  pub fn new(p_title: impl Into<String>) -> Self {
    Self {
      title: p_title.into(),
      width: 1280,
      height: 720,
      resizable: true,
      frame: AppFrame::Bordered,
    }
  }

  /// Sets the inner size in logical pixels.
  pub fn with_size(mut self, p_width: u32, p_height: u32) -> Self {
    self.width = p_width;
    self.height = p_height;
    self
  }

  /// Sets whether the user can resize the window.
  pub fn with_resizable(mut self, p_resizable: bool) -> Self {
    self.resizable = p_resizable;
    self
  }

  /// Sets how the window is framed.
  pub fn with_frame(mut self, p_frame: AppFrame) -> Self {
    self.frame = p_frame;
    self
  }
}

/// An app that owns its window: its options, and the root component its children are added to.
///
/// ```ignore
/// App::new("Editor")
///   .with_size(960, 540)
///   .add(Component::new("page").with_display(Display::Flex).add(toolbar.width(120)).add(viewer.fill()))
///   .run();
/// ```
///
/// The root stacks what is added to it, so one child fills the window. The app has no events of its own: events live on
/// components, and `Quit` sent by any of them ends the app.
pub struct App {
  options: AppOptions,
  root: Component,
}

impl App {
  /// An app with a bordered, resizable 1280x720 window and nothing in it.
  pub fn new(p_title: impl Into<String>) -> Self {
    Self {
      options: AppOptions::new(p_title),
      root: Component::new("app").with_background(Background::Page),
    }
  }

  /// Sets the inner size of the window in logical pixels.
  pub fn with_size(mut self, p_width: u32, p_height: u32) -> Self {
    self.options = self.options.with_size(p_width, p_height);
    self
  }

  /// Sets whether the user can resize the window.
  pub fn with_resizable(mut self, p_resizable: bool) -> Self {
    self.options = self.options.with_resizable(p_resizable);
    self
  }

  /// Sets how the window is framed.
  pub fn with_frame(mut self, p_frame: AppFrame) -> Self {
    self.options = self.options.with_frame(p_frame);
    self
  }

  /// Ends the app when `p_key` is pressed (`"Escape"`, for example), whichever component has the keyboard.
  pub fn with_quit_key(self, p_key: impl Into<String>) -> Self {
    let key = p_key.into();
    let root = self.root.downgrade();
    self.root.subject::<KeyEvent>().subscribe(move |event| {
      if event.pressed && event.key == key {
        if let Some(root) = root.upgrade() {
          root.next(Quit);
        }
      }
    });
    self
  }

  /// Puts `p_child` in the window. Several children are stacked, the last in front; give a child
  /// [`Display::Flex`](crate::Display) or [`Display::Grid`](crate::Display) to place its own children side by side.
  pub fn add(self, p_child: impl Renderable) -> Self {
    self.root.add(p_child);
    self
  }

  /// The window's options so far.
  pub fn options(&self) -> &AppOptions {
    &self.options
  }

  /// The root component the children were added to.
  pub fn root(&self) -> &Component {
    &self.root
  }

  /// Shows the app in a window and returns when it ends. See [`run`].
  ///
  /// Available when the `desktop-window` feature is on.
  #[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
  pub fn run(self) -> Result<(), RunError> {
    run(&self.root, self.options)
  }
}

#[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
pub use desktop_host::{RunError, run};

#[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
mod desktop_host {
  use std::fmt;

  use pub_sub::prelude::*;
  use vessel_engine::surface::desktop::{self, DesktopEvent, WindowFrame, WindowOptions};
  use vessel_engine::{Engine, View};

  use super::{AppFrame, AppOptions};
  use crate::{Component, Font, KeyEvent, PointerButton, PointerEvent, Quit, Theme};

  /// Why the app could not run.
  #[derive(Debug)]
  pub struct RunError(String);

  impl fmt::Display for RunError {
    fn fmt(&self, p_formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(p_formatter, "{}", self.0)
    }
  }

  impl std::error::Error for RunError {}

  /// Shows `p_component` in a window and returns when the app ends: when the user closes the window or the component sends
  /// [`Quit`]. This must be called on the main thread, which the operating system requires of a window.
  ///
  /// The window's size is the component's size, and it follows the window as it is resized. Keyboard and mouse input
  /// reach the component as [`KeyEvent`]s and [`PointerEvent`]s.
  ///
  /// Available when the `desktop-window` feature is on.
  pub fn run(p_component: &Component, p_options: AppOptions) -> Result<(), RunError> {
    let frame = match p_options.frame {
      AppFrame::Bordered => WindowFrame::Bordered,
      AppFrame::Borderless => WindowFrame::Borderless,
      AppFrame::Fullscreen => WindowFrame::FullscreenBorderless,
    };
    let options = WindowOptions::new(p_options.title)
      .with_size(p_options.width, p_options.height)
      .with_resizable(p_options.resizable)
      .with_frame(frame);

    let component = p_component.clone();
    desktop::run(options, move |window| {
      component.size().next(window.size);
      // A window app uses the system's interface font and theme where the app sets none.
      component.set_default_font(Font::system(16));
      component.set_default_theme(Theme::system());
      let engine = Engine::for_surface(window.surface_id);
      let view = View::<()>::new(component.name(), component.clone());
      engine.add(&view);

      // What the platform reports becomes the component's own, platform-neutral events.
      let input = {
        let component = component.clone();
        window.events.subscribe(move |event| match event {
          DesktopEvent::Resized { width, height } => component.size().next((*width, *height)),
          DesktopEvent::Key { key, pressed } => component.next(KeyEvent {
            key: key.clone(),
            pressed: *pressed,
          }),
          DesktopEvent::MouseMoved { x, y } => component.next(PointerEvent::Moved {
            x: *x as f32,
            y: *y as f32,
          }),
          DesktopEvent::MouseButton { button, pressed } => component.next(PointerEvent::Button {
            button: PointerButton::from_name(button),
            pressed: *pressed,
          }),
          DesktopEvent::CloseRequested => {}
        })
      };

      // The component asks to end the app with an event.
      let quit = {
        let window = window.clone();
        // The plain `Observable` subscription, so the listener stops when the window does, not when the component does.
        component.subject::<Quit>().subscribe(move |_: &Quit| window.close())
      };

      // Kept alive until the window closes.
      (engine, view, input, quit)
    })
    .map_err(|error| RunError(error.to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn options_start_bordered_and_can_be_changed() {
    let options = AppOptions::new("App");
    assert_eq!((options.width, options.height), (1280, 720));
    assert_eq!(options.frame, AppFrame::Bordered);
    assert!(options.resizable);

    let options = options.with_size(640, 480).with_resizable(false).with_frame(AppFrame::Fullscreen);
    assert_eq!((options.width, options.height), (640, 480));
    assert!(!options.resizable);
    assert_eq!(options.frame, AppFrame::Fullscreen);
  }

  #[test]
  fn an_app_collects_options_and_adds_children_to_its_root() {
    let child = Component::new("child");
    let app = App::new("Editor").with_size(800, 600).with_resizable(false).with_frame(AppFrame::Borderless).add(&child);
    assert_eq!(app.options().title, "Editor");
    assert_eq!((app.options().width, app.options().height), (800, 600));
    assert!(!app.options().resizable);
    assert_eq!(app.options().frame, AppFrame::Borderless);

    // The root stacks its children, so a single child fills it.
    app.root().size().next((40, 30));
    let mut root = app.root().clone();
    let frame = vessel_engine::MediaSource::render(&mut root, std::time::Duration::ZERO);
    assert_eq!((frame.width, frame.height), (40, 30));
    assert_eq!(child.size().value(), (40, 30));
  }
}
