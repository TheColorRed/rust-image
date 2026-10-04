//! A view engine.
//!
//! An [`Engine`] runs [`View`]s on its own thread. A view shows a [`MediaSource`]: a canvas, a video, a camera, a
//! game, or anything else that can say how often it needs frames, whether it has changed, and how to draw itself. A mini
//! app subscribes to its view's messages and changes its own state; it never says when to draw. The engine decides
//! that, drawing a view that changes only now and then when it has changed and every frame for one that moves on its
//! own.
//!
//! The crate knows nothing about images, canvases or any GPU library: a frame is plain RGBA pixels, or a [`GpuFrame`]
//! that a source rendered on the GPU and a [`GpuTarget`] draws without copying it to the CPU.
//!
//! Where frames are shown is chosen with cargo features, none on by default. `*-surface` features draw inside a view that
//! a host application owns; `*-window` features are for a standalone app or game where Vessel owns the window. Turning a
//! feature on makes the matching surface kind available through [`surface`].
//!
//! `use vessel::prelude::*;` brings in what an app usually needs, including the `pub-sub` traits that `subscribe` and
//! `next` are methods of.

mod engine;
mod gpu;
mod media;
mod view;

pub use pub_sub;
#[cfg(any(feature = "android-surface", feature = "desktop-window"))]
pub use surface;

pub use self::{
  engine::Engine,
  gpu::{GpuFrame, GpuTarget},
  media::{MediaSource, Pacing, Waker},
  view::View,
};

/// A rendered frame of a view.
#[derive(Clone, Debug)]
pub struct Frame {
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
  /// `width * height` RGBA pixels.
  pub pixels: Vec<u8>,
}

/// What an app usually needs, in one import. `subscribe`, `next` and `filter` are trait methods, so the `pub-sub` traits
/// are included. Items for a surface kind are included when its feature is on.
pub mod prelude {
  pub use crate::{Engine, Frame, GpuFrame, GpuTarget, MediaSource, Pacing, View, Waker};
  pub use pub_sub::prelude::*;

  #[cfg(any(feature = "android-surface", feature = "desktop-window"))]
  pub use surface::Surface;
  #[cfg(all(feature = "desktop-window", not(any(target_os = "android", target_os = "ios"))))]
  pub use surface::desktop::{self, DesktopEvent, WindowFrame, WindowOptions};
}
