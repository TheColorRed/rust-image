//! The API that Vessel apps are written against.
//!
//! An app is built from [`Component`]s. A component draws itself on a [`Canvas`], keeps its settings in reactive
//! [`BehaviorSubject`] values, and sends and receives its own typed events. It is built on the `pub-sub` streams: the only verbs
//! are `subscribe` and `next`, and `watch` says which streams should redraw a component.
//!
//! A component is also a `vessel_engine::MediaSource`, so the engine can run it directly. It never knows what platform it is
//! on: it is told its size, and it draws.
//!
//! ```ignore
//! use vessel_api::prelude::*;
//!
//! let level = BehaviorSubject::new(10u8);
//! let swatch = Component::new("swatch")
//!   .draw({ let level = level.clone(); move |canvas| canvas.fill([level.value(), 0, 0, 255]) })
//!   .watch(&level);
//! level.set(200); // redraws the swatch
//! ```
//!
//! The target flags (`android-surface`, `ios-surface`, `desktop-window`) are set on this crate and passed to the engine.
#![deny(missing_docs)]

mod app;
mod canvas;
mod component;
mod font;
mod gpu;
mod input;
mod layout;
#[cfg(any(feature = "android-surface", feature = "ios-surface", feature = "desktop-window"))]
mod mount;
#[cfg(any(feature = "android-surface", feature = "ios-surface", feature = "desktop-window"))]
mod session;
mod shapes;
mod theme;

#[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
pub use self::app::{RunError, run};
#[cfg(any(feature = "android-surface", feature = "ios-surface", feature = "desktop-window"))]
pub use self::mount::{mount, unmount, unmount_all};
#[cfg(any(feature = "android-surface", feature = "ios-surface", feature = "desktop-window"))]
pub use self::session::{Drawn, Session};
pub use self::{
  app::{App, AppFrame, AppOptions},
  canvas::Canvas,
  component::{Component, Renderable, WeakComponent},
  font::{Face, Font, TextAlign},
  gpu::GpuPicture,
  input::{KeyEvent, PointerButton, PointerEvent, Quit},
  layout::{Align, Direction, Display, Edges, Justify, Rect, Track, Units},
  theme::{Background, Theme, ThemePatch},
};

/// What an app usually needs, in one import: this crate's types, the engine's, and the `pub-sub` traits that `subscribe`
/// and `next` are methods of.
pub mod prelude {
  #[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
  pub use crate::run;
  pub use crate::{
    Align, App, AppFrame, AppOptions, Background, Canvas, Component, Direction, Display, Edges, Face, Font, GpuPicture,
    Justify, KeyEvent, PointerButton, PointerEvent, Quit, Renderable, TextAlign, Theme, ThemePatch, Track, Units,
    WeakComponent,
  };
  #[cfg(any(feature = "android-surface", feature = "ios-surface", feature = "desktop-window"))]
  pub use crate::{Drawn, Session, mount, unmount};
  pub use vessel_engine::prelude::*;
}
