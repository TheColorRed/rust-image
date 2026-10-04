//! Frames that live on the GPU, and places to draw them without ever copying them to the CPU.
//!
//! A source that renders on the GPU hands the engine a [`GpuFrame`] instead of pixels. A surface with a [`GpuTarget`] draws
//! it straight from GPU memory. A surface without one asks the frame to [`read_back`](GpuFrame::read_back) into pixels, so
//! every source still works everywhere.
//!
//! The crate knows nothing about any GPU library: a frame is whatever the source and the target agree on, found again with
//! [`as_any`](GpuFrame::as_any).

use std::any::Any;
use std::sync::Arc;

use crate::Frame;

/// A finished picture that lives in GPU memory.
pub trait GpuFrame: Send + Sync + 'static {
  /// The picture's width and height in pixels.
  fn size(&self) -> (u32, u32);

  /// This frame as `Any`, so a [`GpuTarget`] that knows its kind can get at it.
  fn as_any(&self) -> &dyn Any;

  /// Copies the picture to the CPU, for a surface that cannot show it from GPU memory. Slow, and `None` when the frame
  /// cannot be read back.
  fn read_back(&self) -> Option<Frame> {
    None
  }

  /// The target that draws frames like this one on the surface registered under `p_surface_id`, straight from GPU memory,
  /// or `None` if there is none for that surface. A session asks the first frame it shows, so the app never sets a target
  /// up: the frame knows which GPU made it, and the session knows the surface.
  fn target_for(&self, _surface_id: i32) -> Option<Box<dyn GpuTarget>> {
    None
  }
}

/// Somewhere GPU frames are drawn without being copied to the CPU. Once a surface has a target, everything shown on it goes
/// through the target, because a window's GPU swap chain and a CPU blit cannot share it.
pub trait GpuTarget: Send {
  /// Draws a GPU frame. Returns whether it reached the surface; `false` means the surface is gone.
  fn present(&mut self, p_frame: &dyn GpuFrame) -> bool;

  /// Draws plain pixels, for a frame that was made on the CPU. Returns whether it reached the surface.
  fn present_pixels(&mut self, p_frame: &Frame) -> bool;
}

/// What a view made for the engine to draw.
pub(crate) enum Rendered {
  /// Pixels in CPU memory.
  Pixels(Frame),
  /// A picture in GPU memory.
  Gpu(Arc<dyn GpuFrame>),
}
