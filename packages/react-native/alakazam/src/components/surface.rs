//! What the picker surfaces (the curve plot, the gradient bar, the color square) have in common: abra makes the picture and the Vessel engine
//! renders it, so there is no drawing code here, only the plumbing between the two.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicU64, Ordering},
};

use abra::abra_core::{Area, Color, Fill, Gradient, Image as Pixels, ImageRepeat, LineCap, Path};
use abra::drawing::prelude::fill;
use vessel::prelude::*;

use crate::components::frame_of;

/// What a surface shows: the state that a finger and the app change, and a version of it for the engine, which draws a new picture whenever the
/// version is a new number.
pub(crate) struct Shared<S> {
  state: Mutex<S>,
  version: AtomicU64,
  renderer: Mutex<Option<WeakOffscreen>>,
}

impl<S: Clone> Shared<S> {
  pub(crate) fn new(p_state: S) -> Arc<Self> {
    Arc::new(Self { state: Mutex::new(p_state), version: AtomicU64::new(0), renderer: Mutex::new(None) })
  }

  /// The state now.
  pub(crate) fn get(&self) -> S {
    self.state.lock().unwrap().clone()
  }

  /// Changes the state, and has the engine draw it.
  pub(crate) fn update(&self, p_change: impl FnOnce(&mut S)) {
    p_change(&mut self.state.lock().unwrap());
    self.version.fetch_add(1, Ordering::SeqCst);
    let renderer = self.renderer.lock().unwrap().as_ref().and_then(WeakOffscreen::upgrade);
    if let Some(renderer) = renderer {
      renderer.wake();
    }
  }
}

/// What a surface's `Image` follows: the picture `paint` makes of the state at the size of the view.
pub(crate) struct PictureSource<S> {
  shared: Arc<Shared<S>>,
  paint: fn(&S, (u32, u32)) -> Pixels,
}

impl<S> PictureSource<S> {
  pub(crate) fn new(p_shared: &Arc<Shared<S>>, p_paint: fn(&S, (u32, u32)) -> Pixels) -> Self {
    Self { shared: Arc::clone(p_shared), paint: p_paint }
  }
}

impl<S: Clone + Send + Sync + 'static> Trackable for PictureSource<S> {
  fn pictures(&self, size: &BehaviorSubject<(u32, u32)>, _viewport: &BehaviorSubject<Viewport>, _rotation: &BehaviorSubject<f32>) -> Subject<Picture> {
    let (shared, paint) = (Arc::clone(&self.shared), self.paint);
    let version = {
      let shared = Arc::clone(&self.shared);
      move || shared.version.load(Ordering::SeqCst)
    };
    let renderer = Offscreen::tracking(size, version, || false, move |size| {
      // The view has no size until it is shown somewhere.
      (size.0 > 0 && size.1 > 0).then(|| RenderedFrame::Pixels(Arc::new(frame_of(&paint(&shared.get(), *size)))))
    });
    *self.shared.renderer.lock().unwrap() = Some(renderer.downgrade());
    renderer.map(|frame| match frame {
      RenderedFrame::Pixels(frame) => Picture::Pixels(Arc::clone(frame)),
      RenderedFrame::Gpu(frame) => Picture::Gpu(Arc::clone(frame)),
    })
  }
}

/// A transparent picture of `p_size`, which a surface is painted on.
pub(crate) fn blank(p_size: (u32, u32)) -> Pixels {
  Pixels::new(p_size.0, p_size.1)
}

/// Fills the rectangle with its top left corner at `p_origin` with `p_color`.
pub(crate) fn rect(p_image: &mut Pixels, p_origin: (f32, f32), p_size: (f32, f32), p_color: Color) {
  fill(Area::rect(p_origin, p_size), Fill::solid(p_color)).apply(p_image);
}

/// Fills the circle with `p_color`.
pub(crate) fn disc(p_image: &mut Pixels, p_center: (f32, f32), p_radius: f32, p_color: Color) {
  fill(Area::circle(p_center, p_radius), Fill::solid(p_color)).apply(p_image);
}

/// Draws a circle as a line `p_width` wide.
pub(crate) fn ring(p_image: &mut Pixels, p_center: (f32, f32), p_radius: f32, p_width: f32, p_color: Color) {
  let circle = Area::circle(p_center, p_radius);
  fill(circle.stroke(p_width), Fill::solid(p_color)).apply(p_image);
}

/// Draws the line through `p_points`, `p_width` wide, with round ends.
pub(crate) fn line(p_image: &mut Pixels, p_points: &[(f32, f32)], p_width: f32, p_color: Color) {
  let mut path = Path::new();
  for (index, point) in p_points.iter().enumerate() {
    if index == 0 {
      path.move_to(*point);
    } else {
      path.line_to(*point);
    }
  }
  fill(path.stroke(p_width).with_cap(LineCap::Round), Fill::solid(p_color)).apply(p_image);
}

/// Fills the rectangle with its top left corner at `p_origin` with `p_gradient`, which runs across it from left to right unless it has a direction of its own.
pub(crate) fn shade(p_image: &mut Pixels, p_origin: (f32, f32), p_size: (f32, f32), p_gradient: &Gradient) {
  fill(Area::rect(p_origin, p_size), p_gradient).apply(p_image);
}

/// Fills the rectangle with a checkerboard of two light grays, with squares `p_cell` pixels across, to show how see-through what is put over it is.
pub(crate) fn checkerboard(p_image: &mut Pixels, p_origin: (f32, f32), p_size: (f32, f32), p_cell: f32) {
  let mut tile = blank(((p_cell * 2.0) as u32, (p_cell * 2.0) as u32));
  rect(&mut tile, (0.0, 0.0), (p_cell * 2.0, p_cell * 2.0), Color::from_rgb(255, 255, 255));
  for corner in [(0.0, 0.0), (p_cell, p_cell)] {
    rect(&mut tile, corner, (p_cell, p_cell), Color::from_rgb(200, 200, 200));
  }
  fill(Area::rect(p_origin, p_size), Fill::image(Arc::new(tile)).with_repeat(ImageRepeat::Both)).apply(p_image);
}

/// A color from a list of numbers: red, green, blue and alpha, each 0 to 255.
///
/// # Panics
/// When there are fewer than four.
pub(crate) fn color_of(p_color: &[u8]) -> Color {
  Color::from_rgba(p_color[0], p_color[1], p_color[2], p_color[3])
}

#[cfg(test)]
pub(crate) fn pixel(p_image: &Pixels, p_x: u32, p_y: u32) -> [u8; 4] {
  let (width, _) = p_image.dimensions::<u32>();
  let at = ((p_y * width + p_x) * 4) as usize;
  p_image.rgba()[at..at + 4].try_into().unwrap()
}
