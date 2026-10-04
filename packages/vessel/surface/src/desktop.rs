//! A native desktop window for a standalone app or game, where Vessel owns the window and the main loop.
//!
//! [`run`] opens the window, calls the app's setup once the window exists, and runs until the window closes. The window
//! is a [`Surface`] registered under the id in [`WindowOptions`], so a renderer draws on it the same way as on any other
//! surface. Frames are drawn with the CPU. Keyboard, mouse and window events arrive as [`DesktopEvent`]s on a `pub-sub`
//! subject, which the app turns into messages for its views.

use std::any::Any;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pub_sub::{Observer, Subject};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::Key;
use winit::window::{Fullscreen, Window, WindowId};

use super::{Surface, destroy, register_surface};

/// How the window is framed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowFrame {
  /// A normal window with a title bar and a border.
  #[default]
  Bordered,
  /// A window with no title bar and no border.
  Borderless,
  /// A borderless window that covers the whole screen, the usual mode for a game.
  FullscreenBorderless,
}

/// What the window should look like. Build it with [`WindowOptions::new`] and the `with_*` methods.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowOptions {
  /// The title shown in the title bar and the task bar.
  pub title: String,
  /// The inner width in logical pixels.
  pub width: u32,
  /// The inner height in logical pixels.
  pub height: u32,
  /// Whether the user can resize the window.
  pub resizable: bool,
  /// How the window is framed.
  pub frame: WindowFrame,
  /// The id the window's surface is registered under.
  pub surface_id: i32,
}

impl WindowOptions {
  /// A bordered, resizable 1280x720 window registered as surface 1.
  pub fn new(p_title: impl Into<String>) -> Self {
    Self {
      title: p_title.into(),
      width: 1280,
      height: 720,
      resizable: true,
      frame: WindowFrame::Bordered,
      surface_id: 1,
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
  pub fn with_frame(mut self, p_frame: WindowFrame) -> Self {
    self.frame = p_frame;
    self
  }

  /// Sets the id the window's surface is registered under.
  pub fn with_surface_id(mut self, p_surface_id: i32) -> Self {
    self.surface_id = p_surface_id;
    self
  }
}

/// Something that happened to the window or to the user's input on it.
#[derive(Clone, Debug, PartialEq)]
pub enum DesktopEvent {
  /// The window's drawing area changed size, in physical pixels.
  Resized {
    /// The new width.
    width: u32,
    /// The new height.
    height: u32,
  },
  /// A key went down or up. `key` is the character typed (`"a"`) or the name of a special key (`"ArrowUp"`, `"Escape"`).
  Key {
    /// The character or the name of the key.
    key: String,
    /// True when the key went down.
    pressed: bool,
  },
  /// The pointer moved over the window, in physical pixels from its top left corner.
  MouseMoved {
    /// Distance from the left edge.
    x: f64,
    /// Distance from the top edge.
    y: f64,
  },
  /// A mouse button went down or up.
  MouseButton {
    /// `"Left"`, `"Right"`, `"Middle"`, `"Back"`, `"Forward"` or `"Other"`.
    button: String,
    /// True when the button went down.
    pressed: bool,
  },
  /// The user asked to close the window. The program ends after this event.
  CloseRequested,
}

/// What the app's setup gets once the window exists. Cloning gives another handle to the same window.
#[derive(Clone)]
pub struct WindowContext {
  /// The id the window's surface is registered under; a renderer draws on it with this.
  pub surface_id: i32,
  /// The size of the window's drawing area in physical pixels when it opened. Later changes arrive as
  /// [`DesktopEvent::Resized`].
  pub size: (u32, u32),
  /// Input and window events. Subscribe to it from the app's views.
  pub events: Subject<DesktopEvent>,
  proxy: EventLoopProxy<UserEvent>,
}

impl WindowContext {
  /// Closes the window and ends [`run`]. Safe to call from any thread, such as an engine's.
  pub fn close(&self) {
    let _ = self.proxy.send_event(UserEvent::Close);
  }
}

/// Why the window could not run.
#[derive(Debug)]
pub struct RunError(String);

impl fmt::Display for RunError {
  fn fmt(&self, p_formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(p_formatter, "desktop window: {}", self.0)
  }
}

impl std::error::Error for RunError {}

enum UserEvent {
  Redraw,
  Close,
}

/// The newest frame a renderer drew, waiting for the window to show it.
struct PixelFrame {
  width: u32,
  height: u32,
  rgba: Vec<u8>,
}

/// What the renderer's thread and the window's thread share.
struct Shared {
  frame: Mutex<Option<PixelFrame>>,
  size: Mutex<(u32, u32)>,
  alive: AtomicBool,
  redraw_pending: AtomicBool,
  proxy: EventLoopProxy<UserEvent>,
}

/// The surface a renderer draws on. The frame is handed to the window's thread, which shows it.
struct DesktopWindow {
  shared: Arc<Shared>,
}

impl Surface for DesktopWindow {
  fn is_alive(&self) -> bool {
    self.shared.alive.load(Ordering::SeqCst)
  }

  fn size(&self) -> (u32, u32) {
    *self.shared.size.lock().unwrap()
  }

  fn resize(&self, p_width: u32, p_height: u32) {
    *self.shared.size.lock().unwrap() = (p_width, p_height);
  }

  fn draw_rgba(&self, p_width: u32, p_height: u32, p_rgba: &[u8]) -> bool {
    if !self.is_alive() || p_rgba.len() < p_width as usize * p_height as usize * 4 {
      return false;
    }
    {
      let mut frame = self.shared.frame.lock().unwrap();
      let frame = frame.get_or_insert_with(|| PixelFrame {
        width: 0,
        height: 0,
        rgba: Vec::new(),
      });
      frame.width = p_width;
      frame.height = p_height;
      frame.rgba.clear();
      frame.rgba.extend_from_slice(&p_rgba[..p_width as usize * p_height as usize * 4]);
    }
    // One request is enough however many frames arrive before the window gets to it.
    if !self.shared.redraw_pending.swap(true, Ordering::SeqCst) {
      let _ = self.shared.proxy.send_event(UserEvent::Redraw);
    }
    true
  }

  fn retire(&self) {
    self.shared.alive.store(false, Ordering::SeqCst);
  }

  fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
    self
  }
}

/// Stretches `p_frame` over `p_target` (`p_width * p_height` pixels of `0x00RRGGBB`) with nearest-neighbour sampling.
fn scale_into(p_target: &mut [u32], p_width: u32, p_height: u32, p_frame: &PixelFrame) {
  if p_frame.width == 0 || p_frame.height == 0 || p_width == 0 || p_height == 0 {
    p_target.fill(0);
    return;
  }
  for y in 0..p_height as usize {
    let source_y = y * p_frame.height as usize / p_height as usize;
    let row = &mut p_target[y * p_width as usize..(y + 1) * p_width as usize];
    for (x, pixel) in row.iter_mut().enumerate() {
      let source_x = x * p_frame.width as usize / p_width as usize;
      let at = (source_y * p_frame.width as usize + source_x) * 4;
      let rgba = &p_frame.rgba[at..at + 4];
      *pixel = (rgba[0] as u32) << 16 | (rgba[1] as u32) << 8 | rgba[2] as u32;
    }
  }
}

type SoftSurface = softbuffer::Surface<Arc<Window>, Arc<Window>>;

struct Runner<T, F: FnOnce(&WindowContext) -> T> {
  options: WindowOptions,
  setup: Option<F>,
  proxy: EventLoopProxy<UserEvent>,
  window: Option<Arc<Window>>,
  soft_surface: Option<SoftSurface>,
  shared: Option<Arc<Shared>>,
  events: Subject<DesktopEvent>,
  /// Whatever setup returned, kept until the window closes, such as an engine that must keep running.
  keep: Option<T>,
  error: Option<String>,
}

impl<T, F: FnOnce(&WindowContext) -> T> Runner<T, F> {
  fn open(&mut self, p_event_loop: &ActiveEventLoop) -> Result<(), String> {
    let mut attributes = Window::default_attributes()
      .with_title(self.options.title.clone())
      .with_inner_size(LogicalSize::new(self.options.width, self.options.height))
      .with_resizable(self.options.resizable);
    match self.options.frame {
      WindowFrame::Bordered => {}
      WindowFrame::Borderless => attributes = attributes.with_decorations(false),
      WindowFrame::FullscreenBorderless => {
        attributes = attributes.with_decorations(false).with_fullscreen(Some(Fullscreen::Borderless(None)));
      }
    }
    let window = Arc::new(p_event_loop.create_window(attributes).map_err(|error| error.to_string())?);
    let context = softbuffer::Context::new(window.clone()).map_err(|error| error.to_string())?;
    let soft_surface = softbuffer::Surface::new(&context, window.clone()).map_err(|error| error.to_string())?;

    let size = window.inner_size();
    let shared = Arc::new(Shared {
      frame: Mutex::new(None),
      size: Mutex::new((size.width, size.height)),
      alive: AtomicBool::new(true),
      redraw_pending: AtomicBool::new(false),
      proxy: self.proxy.clone(),
    });
    register_surface(self.options.surface_id, Arc::new(DesktopWindow { shared: shared.clone() }));

    self.window = Some(window);
    self.soft_surface = Some(soft_surface);
    self.shared = Some(shared);

    if let Some(setup) = self.setup.take() {
      let context = WindowContext {
        surface_id: self.options.surface_id,
        size: (size.width, size.height),
        events: self.events.clone(),
        proxy: self.proxy.clone(),
      };
      self.keep = Some(setup(&context));
    }
    Ok(())
  }

  fn redraw(&mut self) {
    let (Some(window), Some(soft_surface), Some(shared)) = (&self.window, &mut self.soft_surface, &self.shared) else {
      return;
    };
    // Cleared first, so a frame that arrives while this one is shown asks for another redraw.
    shared.redraw_pending.store(false, Ordering::SeqCst);
    let size = window.inner_size();
    let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
      return;
    };
    if soft_surface.resize(width, height).is_err() {
      return;
    }
    let Ok(mut buffer) = soft_surface.buffer_mut() else { return };
    match shared.frame.lock().unwrap().as_ref() {
      Some(frame) => scale_into(&mut buffer, size.width, size.height, frame),
      None => buffer.fill(0),
    }
    let _ = buffer.present();
  }
}

impl<T, F: FnOnce(&WindowContext) -> T> ApplicationHandler<UserEvent> for Runner<T, F> {
  fn resumed(&mut self, p_event_loop: &ActiveEventLoop) {
    if self.window.is_some() {
      return;
    }
    if let Err(error) = self.open(p_event_loop) {
      self.error = Some(error);
      p_event_loop.exit();
    }
  }

  fn user_event(&mut self, p_event_loop: &ActiveEventLoop, p_event: UserEvent) {
    match p_event {
      UserEvent::Redraw => {
        if let Some(window) = &self.window {
          window.request_redraw();
        }
      }
      UserEvent::Close => p_event_loop.exit(),
    }
  }

  fn window_event(&mut self, p_event_loop: &ActiveEventLoop, _p_id: WindowId, p_event: WindowEvent) {
    match p_event {
      WindowEvent::CloseRequested => {
        self.events.next(DesktopEvent::CloseRequested);
        p_event_loop.exit();
      }
      WindowEvent::Resized(size) => {
        if let Some(shared) = &self.shared {
          *shared.size.lock().unwrap() = (size.width, size.height);
        }
        self.events.next(DesktopEvent::Resized {
          width: size.width,
          height: size.height,
        });
        if let Some(window) = &self.window {
          window.request_redraw();
        }
      }
      WindowEvent::RedrawRequested => self.redraw(),
      WindowEvent::KeyboardInput { event, .. } => {
        let key = match &event.logical_key {
          Key::Character(text) => text.to_string(),
          Key::Named(name) => format!("{name:?}"),
          other => format!("{other:?}"),
        };
        self.events.next(DesktopEvent::Key {
          key,
          pressed: event.state == ElementState::Pressed,
        });
      }
      WindowEvent::CursorMoved { position, .. } => {
        self.events.next(DesktopEvent::MouseMoved {
          x: position.x,
          y: position.y,
        });
      }
      WindowEvent::MouseInput { state, button, .. } => {
        self.events.next(DesktopEvent::MouseButton {
          button: format!("{button:?}"),
          pressed: state == ElementState::Pressed,
        });
      }
      _ => {}
    }
  }
}

/// Opens a window and runs until it closes.
///
/// `p_setup` runs once the window exists. Create the engine and the views there, and use [`WindowContext::events`] for
/// input. Whatever it returns is kept alive until the window closes, so return the engine to keep it running.
///
/// This must be called on the main thread, which the operating system requires of a window's event loop.
pub fn run<T: 'static>(p_options: WindowOptions, p_setup: impl FnOnce(&WindowContext) -> T) -> Result<(), RunError> {
  let event_loop = EventLoop::<UserEvent>::with_user_event().build().map_err(|error| RunError(error.to_string()))?;
  event_loop.set_control_flow(ControlFlow::Wait);
  let surface_id = p_options.surface_id;
  let mut runner = Runner {
    options: p_options,
    setup: Some(p_setup),
    proxy: event_loop.create_proxy(),
    window: None,
    soft_surface: None,
    shared: None,
    events: Subject::new(),
    keep: None,
    error: None,
  };
  let result = event_loop.run_app(&mut runner).map_err(|error| RunError(error.to_string()));
  destroy(surface_id);
  result?;
  match runner.error {
    Some(error) => Err(RunError(error)),
    None => Ok(()),
  }
}

/// How the user has set up their desktop: dark or light apps, and their accent color. Read when asked; a program that
/// wants to follow changes asks again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Appearance {
  /// Whether the user prefers dark apps.
  pub dark: bool,
  /// The user's accent color as red, green and blue, if the system has one.
  pub accent: Option<[u8; 3]>,
}

/// The user's appearance settings. Windows only for now: elsewhere it is the default (light, no accent).
pub fn appearance() -> Appearance {
  #[cfg(windows)]
  {
    const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    const DWM: &str = r"Software\Microsoft\Windows\DWM";
    let dark = windows_registry::dword(PERSONALIZE, "AppsUseLightTheme") == Some(0);
    // `AccentColor` is stored as 0xAABBGGRR.
    let accent = windows_registry::dword(DWM, "AccentColor").map(|value| [value as u8, (value >> 8) as u8, (value >> 16) as u8]);
    Appearance { dark, accent }
  }
  #[cfg(not(windows))]
  {
    Appearance::default()
  }
}

/// Reads values from the Windows registry. Only what is needed, so no crate is pulled in for it.
#[cfg(windows)]
mod windows_registry {
  use std::ffi::c_void;

  #[link(name = "advapi32")]
  unsafe extern "system" {
    fn RegGetValueW(
      p_key: isize,
      p_sub_key: *const u16,
      p_value: *const u16,
      p_flags: u32,
      p_type: *mut u32,
      p_data: *mut c_void,
      p_size: *mut u32,
    ) -> i32;
  }

  /// The handle `HKEY_CURRENT_USER` stands for: 0x80000001, sign-extended.
  const CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
  /// Only accept a 32-bit number.
  const REG_DWORD_ONLY: u32 = 0x0000_0010;

  /// The 32-bit number stored under `p_sub_key` of the current user's registry as `p_name`, if there is one.
  pub fn dword(p_sub_key: &str, p_name: &str) -> Option<u32> {
    let wide = |text: &str| text.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (sub_key, name) = (wide(p_sub_key), wide(p_name));
    let (mut value, mut size) = (0u32, 4u32);
    // SAFETY: the strings end in zero and live through the call, and `value` has the 4 bytes `size` says.
    let status = unsafe {
      RegGetValueW(
        CURRENT_USER,
        sub_key.as_ptr(),
        name.as_ptr(),
        REG_DWORD_ONLY,
        std::ptr::null_mut(),
        (&mut value as *mut u32).cast(),
        &mut size,
      )
    };
    (status == 0).then_some(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn frame(p_width: u32, p_height: u32, p_pixels: &[[u8; 4]]) -> PixelFrame {
    PixelFrame {
      width: p_width,
      height: p_height,
      rgba: p_pixels.iter().flatten().copied().collect(),
    }
  }

  #[test]
  fn options_start_bordered_and_can_be_changed() {
    let options = WindowOptions::new("Game");
    assert_eq!((options.width, options.height), (1280, 720));
    assert_eq!(options.frame, WindowFrame::Bordered);
    assert!(options.resizable);

    let options = options.with_size(640, 480).with_frame(WindowFrame::Borderless).with_resizable(false).with_surface_id(7);
    assert_eq!((options.width, options.height), (640, 480));
    assert_eq!(options.frame, WindowFrame::Borderless);
    assert!(!options.resizable);
    assert_eq!(options.surface_id, 7);
  }

  #[test]
  fn a_frame_is_stretched_over_the_window() {
    // 2x1: red then blue, stretched to 4x2.
    let source = frame(2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let mut target = vec![0u32; 8];
    scale_into(&mut target, 4, 2, &source);
    let (red, blue) = (0x00FF0000, 0x000000FF);
    assert_eq!(target, vec![red, red, blue, blue, red, red, blue, blue]);
  }

  #[test]
  fn a_frame_larger_than_the_window_is_sampled_down() {
    let source = frame(4, 1, &[[1, 0, 0, 255], [2, 0, 0, 255], [3, 0, 0, 255], [4, 0, 0, 255]]);
    let mut target = vec![0u32; 2];
    scale_into(&mut target, 2, 1, &source);
    assert_eq!(target, vec![0x00010000, 0x00030000]);
  }

  #[test]
  fn an_empty_frame_clears_the_window() {
    let mut target = vec![7u32; 4];
    scale_into(&mut target, 2, 2, &frame(0, 0, &[]));
    assert_eq!(target, vec![0; 4]);
  }

  #[test]
  fn appearance_can_be_read_without_failing() {
    // Whatever the machine has set, asking never panics, and an accent has three channels.
    let appearance = appearance();
    assert!(appearance.accent.map_or(true, |accent| accent.len() == 3));
  }
}
