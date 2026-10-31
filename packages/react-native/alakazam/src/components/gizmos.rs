//! Gizmos: lines, outlines and shapes drawn over the photo, as a component that React creates and the Vessel engine draws.
//!
//! `Gizmos` is a transparent picture the size of its view, with a list of gizmos on it. React adds, changes and removes them by
//! name (`set_line("lasso", ..)`, `remove("lasso")`) and the library draws them (`drawing`: a shader for a line, the area
//! painters for an outline or a fill), so the app has no drawing code. Put its `VesselView` over the photo's. Gizmos are drawn in
//! the order they were first added, the newest on top, and a name that is set again keeps its place.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, AtomicU64, Ordering},
};

use abra::abra_core::{
  BlendMode, Image as Pixels,
  image::recipe::blended,
};
use abra::drawing::prelude::{Brush, StrokeLook, draw_area_fill, draw_area_stroke, stroke_overlay_deferred};
use gpu_passes::strokes::{StrokeCanvas, StrokeStyle};
use vessel::prelude::*;

use crate::components::frame_of;

/// One thing drawn over the photo. Points and areas are in pixels of the view.
enum Gizmo {
  /// A line through points, such as the loop a finger is drawing. `drawn` is how many of the points are already on the GPU's canvas.
  Line { points: Vec<(f32, f32)>, brush: Brush<'static>, drawn: usize },
  /// The edge of an area, as wide as the brush.
  Outline(abra::prelude::Area, Brush<'static>),
  /// The inside of an area.
  Fill(abra::prelude::Area, Brush<'static>),
  /// A frame round a four-cornered shape, such as the box round a selected picture. Its corners are in pixels of the photo, not of the
  /// view, so it stays on the photo while the photo is zoomed, moved or turned (see [`Camera`]). `drawn` is whether the GPU's canvas
  /// already has it.
  Box { corners: [(f32, f32); 4], brush: Brush<'static>, drawn: bool },
}

/// Where the view looks at the photo, so a gizmo placed on the photo is drawn where that part of the photo is on the screen. Set with the
/// same numbers as the photo's own view, so the two always agree.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Camera {
  /// The size of the photo in pixels.
  photo: (f32, f32),
  /// The part of the photo the view shows, as shares of the photo: left, top, width, height. It can reach past the photo's edges.
  viewport: [f32; 4],
  /// How far the photo is turned, clockwise, in radians, about the middle of the view.
  angle: f32,
}

impl Camera {
  /// A point of the photo, in photo pixels, as a point of a view of `size` pixels.
  fn to_view(&self, p_point: (f32, f32), p_size: (u32, u32)) -> (f32, f32) {
    let (width, height) = (p_size.0 as f32, p_size.1 as f32);
    let across = (p_point.0 / self.photo.0 - self.viewport[0]) / self.viewport[2] * width - width / 2.0;
    let down = (p_point.1 / self.photo.1 - self.viewport[1]) / self.viewport[3] * height - height / 2.0;
    let (sin, cos) = self.angle.sin_cos();
    (width / 2.0 + across * cos - down * sin, height / 2.0 + across * sin + down * cos)
  }
}

/// The corners of a box on the photo as points of the view: through the camera when there is one, and as they are when there is not.
fn box_in_view(p_corners: &[(f32, f32); 4], p_camera: Option<Camera>, p_size: (u32, u32)) -> [(f32, f32); 4] {
  p_corners.map(|corner| p_camera.map_or(corner, |camera| camera.to_view(corner, p_size)))
}

impl Gizmo {
  /// The gizmo as a picture of `width` x `height` pixels: transparent everywhere except where it is.
  fn layer(&self, width: u32, height: u32, p_camera: Option<Camera>) -> Pixels {
    match self {
      Gizmo::Box { corners, brush, .. } => {
        let [first, second, third, fourth] = box_in_view(corners, p_camera, (width, height));
        stroke_overlay_deferred(width, height, &[first, second, third, fourth, first], brush)
      }
      Gizmo::Line { points, brush, .. } => stroke_overlay_deferred(width, height, points, brush),
      Gizmo::Outline(area, brush) => {
        let mut layer = Pixels::new(width, height);
        draw_area_stroke(&mut layer, area, brush);
        layer
      }
      Gizmo::Fill(area, brush) => {
        let mut layer = Pixels::new(width, height);
        draw_area_fill(&mut layer, area, brush);
        layer
      }
    }
  }
}

/// The gizmos, shared by the component and the pictures made from it.
struct GizmoList {
  gizmos: Mutex<Vec<(String, Gizmo)>>,
  /// Changes whenever the gizmos do, so the renderer knows there is something new to draw.
  version: AtomicU64,
  /// Whether the GPU's canvas no longer matches the lines (one was replaced or taken away), so it is drawn again from the start.
  rebuild: AtomicBool,
  /// The renderer that makes the pictures, once there is one, so it can be woken when a gizmo changes. Weak, because the renderer's
  /// drawing code holds this list, and a strong link back would keep the renderer and its last frame alive after the component is gone.
  renderer: Mutex<Option<WeakOffscreen>>,
  /// Where the view looks at the photo, once the photo's view says. Gizmos placed on the photo wait for it.
  camera: Mutex<Option<Camera>>,
}

impl GizmoList {
  /// Every gizmo over a clear picture of `width` x `height` pixels, the newest on top.
  fn frame(&self, width: u32, height: u32) -> Frame {
    let gizmos = self.gizmos.lock().unwrap();
    let camera = *self.camera.lock().unwrap();
    let mut picture = Pixels::new(width, height);
    for (_, gizmo) in gizmos.iter() {
      picture = blended(&picture, &gizmo.layer(width, height, camera), BlendMode::Normal, 1.0);
    }
    frame_of(&picture)
  }

  /// Adds the gizmo, or replaces the one with this name where it is in the list, and draws again.
  fn set(&self, name: String, gizmo: Gizmo) {
    let mut gizmos = self.gizmos.lock().unwrap();
    match gizmos.iter_mut().find(|(existing, _)| *existing == name) {
      Some((_, existing)) => {
        *existing = gizmo;
        self.rebuild.store(true, Ordering::SeqCst);
      }
      None => gizmos.push((name, gizmo)),
    }
    drop(gizmos);
    self.changed();
  }

  /// Moves the camera. A frame that was drawn with the old one is wrong, so everything is drawn again.
  fn look(&self, p_camera: Camera) {
    let mut camera = self.camera.lock().unwrap();
    if *camera == Some(p_camera) {
      return;
    }
    *camera = Some(p_camera);
    drop(camera);
    // Only gizmos on the photo move with the camera, so a view with just lines (a finger's loop) is not drawn again for nothing.
    if self.gizmos.lock().unwrap().iter().any(|(_, gizmo)| matches!(gizmo, Gizmo::Box { .. })) {
      self.rebuild.store(true, Ordering::SeqCst);
      self.changed();
    }
  }

  fn changed(&self) {
    self.version.fetch_add(1, Ordering::SeqCst);
    let renderer = self.renderer.lock().unwrap().as_ref().and_then(WeakOffscreen::upgrade);
    if let Some(renderer) = renderer {
      renderer.wake();
    }
  }
}

/// The canvas the lines are drawn on, kept for as long as the view is the same size so a frame draws only what is new.
struct OnTheGpu {
  canvas: StrokeCanvas,
}

impl GizmoList {
  /// The lines drawn by the GPU into a texture that goes to the screen without being copied to the CPU. Only the segments that are
  /// new since the last frame are drawn, so a frame costs the length of what a finger added and not the size of the view. `None`
  /// when there is nothing to draw, when a gizmo is not a line (an outline or a fill is made on the CPU), or when there is no GPU:
  /// the caller draws the frame on the CPU then.
  fn gpu_frame(&self, size: (u32, u32), held: &mut Option<OnTheGpu>) -> Option<RenderedFrame> {
    let mut gizmos = self.gizmos.lock().unwrap();
    if gizmos.is_empty() || gizmos.iter().any(|(_, gizmo)| !matches!(gizmo, Gizmo::Line { .. } | Gizmo::Box { .. })) {
      self.rebuild.store(true, Ordering::SeqCst);
      return None;
    }
    let context = gpu::context()?;
    if held.as_ref().is_none_or(|on_the_gpu| on_the_gpu.canvas.size() != size) {
      *held = Some(OnTheGpu { canvas: StrokeCanvas::new(context, size.0, size.1) });
      self.rebuild.store(true, Ordering::SeqCst);
    }
    let canvas = &held.as_ref()?.canvas;
    let rebuild = self.rebuild.swap(false, Ordering::SeqCst);
    if rebuild {
      canvas.clear();
    }
    let camera = *self.camera.lock().unwrap();
    for (_, gizmo) in gizmos.iter_mut() {
      if let Gizmo::Box { corners, brush, drawn } = gizmo {
        if rebuild {
          *drawn = false;
        }
        if !*drawn {
          let [first, second, third, fourth] = box_in_view(corners, camera, size);
          let edges = [[first.0, first.1, second.0, second.1], [second.0, second.1, third.0, third.1], [third.0, third.1, fourth.0, fourth.1], [fourth.0, fourth.1, first.0, first.1]];
          let look = StrokeLook::of(brush);
          canvas.draw(&edges, &StrokeStyle { radius: look.radius, edge: look.edge, color: look.color });
          *drawn = true;
        }
        continue;
      }
      let Gizmo::Line { points, brush, drawn } = gizmo else { continue };
      if rebuild {
        *drawn = 0;
      }
      // A line with one point is a dot, and every point after the first adds the segment from the one before.
      let segments: Vec<[f32; 4]> = match (*drawn, points.len()) {
        (_, 0) => Vec::new(),
        (0, 1) => vec![[points[0].0, points[0].1, points[0].0, points[0].1]],
        (drawn, _) => points
          .windows(2)
          .skip(drawn.saturating_sub(1))
          .map(|pair| [pair[0].0, pair[0].1, pair[1].0, pair[1].1])
          .collect(),
      };
      let look = StrokeLook::of(brush);
      canvas.draw(&segments, &StrokeStyle { radius: look.radius, edge: look.edge, color: look.color });
      *drawn = points.len();
    }
    Some(RenderedFrame::Gpu(Arc::new(gpu_passes::target::TextureFrame::new(context, canvas.texture().clone()))))
  }
}

impl GizmoList {
  /// The frame for a view of `size`, or nothing when there is nothing to draw and nothing on screen to clear. `visible` is whether the
  /// last frame had something on it. A view with no gizmos makes no frame at all, so the view stays empty without a transparent picture
  /// the size of the screen being made, kept and scaled each time the view changes size. When the last gizmo is taken away, one clear
  /// frame replaces what was drawn, and the GPU canvas that held it is let go.
  fn next_frame(&self, size: (u32, u32), held: &mut Option<OnTheGpu>, visible: &mut bool) -> Option<RenderedFrame> {
    if size.0 == 0 || size.1 == 0 {
      return None;
    }
    if self.gizmos.lock().unwrap().is_empty() {
      if !*visible {
        return None;
      }
      *visible = false;
      *held = None;
      return Some(RenderedFrame::Pixels(Arc::new(self.frame(size.0, size.1))));
    }
    *visible = true;
    if let Some(frame) = self.gpu_frame(size, held) {
      return Some(frame);
    }
    // Drawn on the CPU, so the GPU canvas of an earlier line is not used and is let go (it is made again when only lines are left).
    *held = None;
    Some(RenderedFrame::Pixels(Arc::new(self.frame(size.0, size.1))))
  }
}

/// What the component's `Image` follows.
struct GizmoSource(Arc<GizmoList>);

impl Trackable for GizmoSource {
  fn pictures(&self, size: &BehaviorSubject<(u32, u32)>, _viewport: &BehaviorSubject<Viewport>, _rotation: &BehaviorSubject<f32>) -> Subject<Picture> {
    let list = Arc::clone(&self.0);
    let version = {
      let list = Arc::clone(&self.0);
      move || list.version.load(Ordering::SeqCst)
    };
    let mut held: Option<OnTheGpu> = None;
    let mut visible = false;
    let renderer = Offscreen::tracking(size, version, || false, move |size| list.next_frame(*size, &mut held, &mut visible));
    *self.0.renderer.lock().unwrap() = Some(renderer.downgrade());
    renderer.map(|frame| match frame {
      RenderedFrame::Pixels(frame) => Picture::Pixels(Arc::clone(frame)),
      RenderedFrame::Gpu(frame) => Picture::Gpu(Arc::clone(frame)),
    })
  }
}

/// Gizmos over the photo. Sizes and positions are in the pixels of the view, not in points, so a screen with a density of 3 wants
/// a brush three times as big and points three times as far apart.
#[derive(uniffi::Object, Component)]
pub struct Gizmos {
  image: Image,
  list: Arc<GizmoList>,
}

#[uniffi::export]
impl Gizmos {
  /// A view with no gizmos on it.
  #[uniffi::constructor]
  pub fn new() -> Arc<Self> {
    let list = Arc::new(GizmoList { gizmos: Mutex::new(Vec::new()), version: AtomicU64::new(0), rebuild: AtomicBool::new(true), renderer: Mutex::new(None), camera: Mutex::new(None) });
    let image = Image::new().track(&GizmoSource(Arc::clone(&list)));
    Arc::new(Self { image, list })
  }

  /// Draws a line through `points` (each `[x, y]`) with the brush, as the gizmo `name`. Panics when a point is not two numbers.
  pub fn set_line(&self, name: String, points: Vec<Vec<f32>>, brush: Arc<abra_uniffi::Brush>) {
    let points = points
      .iter()
      .map(|point| {
        let [x, y] = point[..] else { panic!("a point is [x, y], not {} numbers", point.len()) };
        (x, y)
      })
      .collect();
    self.list.set(name, Gizmo::Line { points, brush: brush.0.clone(), drawn: 0 });
  }

  /// Adds a point to the end of the line `name`, which is a new line with this point alone if there is none, drawn with the
  /// brush. Each point of a line that a finger is drawing is one call, and the points already sent are not sent again.
  pub fn add_point(&self, name: String, x: f32, y: f32, brush: Arc<abra_uniffi::Brush>) {
    let mut gizmos = self.list.gizmos.lock().unwrap();
    match gizmos.iter_mut().find(|(existing, _)| *existing == name) {
      Some((_, Gizmo::Line { points, .. })) => points.push((x, y)),
      Some((_, other)) => {
        *other = Gizmo::Line { points: vec![(x, y)], brush: brush.0.clone(), drawn: 0 };
        self.list.rebuild.store(true, Ordering::SeqCst);
      }
      None => gizmos.push((name, Gizmo::Line { points: vec![(x, y)], brush: brush.0.clone(), drawn: 0 })),
    }
    drop(gizmos);
    self.list.changed();
  }

  /// Draws the edge of `area`, as wide as the brush, as the gizmo `name`.
  pub fn set_outline(&self, name: String, area: Arc<abra_uniffi::Area>, brush: Arc<abra_uniffi::Brush>) {
    self.list.set(name, Gizmo::Outline(area.core(), brush.0.clone()));
  }

  /// Fills `area` with the brush, as the gizmo `name`.
  pub fn set_fill(&self, name: String, area: Arc<abra_uniffi::Area>, brush: Arc<abra_uniffi::Brush>) {
    self.list.set(name, Gizmo::Fill(area.core(), brush.0.clone()));
  }

  /// Draws a frame round the four corners (each `[x, y]`, top left, top right, bottom right, bottom left) with the brush, as the gizmo
  /// `name`. The corners are in pixels of the photo, not of the view: the frame stays on the photo as the camera (`set_camera`) moves.
  /// Panics when there are not four corners of two numbers each.
  pub fn set_box(&self, name: String, corners: Vec<Vec<f32>>, brush: Arc<abra_uniffi::Brush>) {
    let points: Vec<(f32, f32)> = corners
      .iter()
      .map(|point| {
        let [x, y] = point[..] else { panic!("a corner is [x, y], not {} numbers", point.len()) };
        (x, y)
      })
      .collect();
    let corners: [(f32, f32); 4] = points.try_into().unwrap_or_else(|points: Vec<(f32, f32)>| panic!("a box has four corners, not {}", points.len()));
    self.list.set(name, Gizmo::Box { corners, brush: brush.0.clone(), drawn: false });
  }

  /// Tells the gizmos where the view looks at the photo, with the same numbers the photo's own view is given: the photo's size in
  /// pixels, the part of it that shows (`left`, `top`, `width`, `height`, each as a share of the photo) and how far it is turned,
  /// clockwise, in radians. Call it whenever the camera moves; gizmos placed on the photo (`set_box`) follow.
  pub fn set_camera(&self, photo_width: f32, photo_height: f32, left: f32, top: f32, width: f32, height: f32, angle: f32) {
    if photo_width > 0.0 && photo_height > 0.0 && width > 0.0 && height > 0.0 {
      self.list.look(Camera { photo: (photo_width, photo_height), viewport: [left, top, width, height], angle });
    }
  }

  /// Takes the gizmo `name` away, if there is one.
  pub fn remove(&self, name: String) {
    self.list.gizmos.lock().unwrap().retain(|(existing, _)| *existing != name);
    self.list.rebuild.store(true, Ordering::SeqCst);
    self.list.changed();
  }

  /// Takes every gizmo away, leaving the view clear.
  pub fn clear(&self) {
    self.list.gizmos.lock().unwrap().clear();
    self.list.rebuild.store(true, Ordering::SeqCst);
    self.list.changed();
  }

  /// How many points the line `name` has, or 0 when there is no such line.
  pub fn point_count(&self, name: String) -> u32 {
    match self.list.gizmos.lock().unwrap().iter().find(|(existing, _)| *existing == name) {
      Some((_, Gizmo::Line { points, .. })) => points.len() as u32,
      _ => 0,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::Color;

  fn list() -> GizmoList {
    GizmoList { gizmos: Mutex::new(Vec::new()), version: AtomicU64::new(0), rebuild: AtomicBool::new(true), renderer: Mutex::new(None), camera: Mutex::new(None) }
  }

  fn brush(color: &Color, size: u32) -> Brush<'static> {
    Brush::new().with_size(size).with_hardness(1.0).with_color(color)
  }

  fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
    let at = (y * frame.width as usize + x) * 4;
    [frame.pixels[at], frame.pixels[at + 1], frame.pixels[at + 2], frame.pixels[at + 3]]
  }

  #[test]
  fn the_camera_puts_a_point_of_the_photo_where_it_is_on_the_screen() {
    // A 200 x 100 photo seen whole in a 400 x 200 view: everything is twice as far from the corner.
    let whole = Camera { photo: (200.0, 100.0), viewport: [0.0, 0.0, 1.0, 1.0], angle: 0.0 };
    assert_eq!(whole.to_view((50.0, 25.0), (400, 200)), (100.0, 50.0));
    // Zoomed to its right half, the middle of the photo is the left edge of the view.
    let right_half = Camera { viewport: [0.5, 0.0, 0.5, 1.0], ..whole };
    assert_eq!(right_half.to_view((100.0, 50.0), (400, 200)), (0.0, 100.0));
    // Turned a quarter, clockwise, about the middle of the view: the top-left corner goes to the top-right.
    let turned = Camera { angle: std::f32::consts::FRAC_PI_2, ..Camera { photo: (100.0, 100.0), viewport: [0.0, 0.0, 1.0, 1.0], angle: 0.0 } };
    let (x, y) = turned.to_view((0.0, 0.0), (100, 100));
    assert!((x - 100.0).abs() < 1e-3 && y.abs() < 1e-3, "{x} {y}");
  }

  #[test]
  fn a_box_follows_the_camera_and_is_drawn_on_the_cpu_without_a_gpu() {
    let list = list();
    let color = Color::from_rgba(255, 0, 0, 255);
    let corners = [(10.0, 10.0), (30.0, 10.0), (30.0, 20.0), (10.0, 20.0)];
    list.set("box".into(), Gizmo::Box { corners, brush: brush(&color, 2), drawn: false });
    // With the photo seen whole at the view's size, the frame is where its corners say.
    list.look(Camera { photo: (100.0, 50.0), viewport: [0.0, 0.0, 1.0, 1.0], angle: 0.0 });
    let frame = list.frame(100, 50);
    assert_eq!(pixel(&frame, 20, 10)[3], 255, "the top edge");
    assert_eq!(pixel(&frame, 20, 15)[3], 0, "inside the frame is clear");
    // The camera moves to the right half of the photo: the same frame is now off the left of the view, and a box that is not in view
    // draws nothing.
    list.look(Camera { photo: (100.0, 50.0), viewport: [0.5, 0.0, 0.5, 1.0], angle: 0.0 });
    let moved = list.frame(100, 50);
    assert_eq!(pixel(&moved, 20, 10)[3], 0, "the frame moved with the photo");
  }

  #[test]
  fn a_view_with_no_gizmos_makes_no_frame() {
    let list = list();
    let (mut held, mut visible) = (None, false);
    assert!(list.next_frame((40, 20), &mut held, &mut visible).is_none(), "nothing to draw and nothing to clear");
    assert!(list.next_frame((80, 40), &mut held, &mut visible).is_none(), "a new size changes nothing");
    assert!(list.next_frame((0, 20), &mut held, &mut visible).is_none(), "a view with no size has no frame");
    assert!(!visible);
  }

  #[test]
  fn taking_the_last_gizmo_away_makes_one_clear_frame_and_then_none() {
    let list = list();
    let (mut held, mut visible) = (None, false);
    list.set("ring".into(), Gizmo::Outline(abra::prelude::Area::circle((20.0, 10.0), 8.0), brush(&Color::white(), 3)));
    let Some(RenderedFrame::Pixels(drawn)) = list.next_frame((40, 20), &mut held, &mut visible) else { panic!("a frame of pixels") };
    assert!(visible);
    assert!(drawn.pixels.chunks(4).any(|pixel| pixel[3] > 0), "the ring is drawn");

    list.gizmos.lock().unwrap().retain(|(name, _)| name != "ring");
    let Some(RenderedFrame::Pixels(cleared)) = list.next_frame((40, 20), &mut held, &mut visible) else { panic!("a clear frame") };
    assert!(!visible);
    assert!(held.is_none(), "the GPU canvas is let go");
    assert!(cleared.pixels.chunks(4).all(|pixel| pixel[3] == 0), "nothing is left on the view");
    assert!(list.next_frame((40, 20), &mut held, &mut visible).is_none(), "and then nothing more is made");
    assert!(list.next_frame((80, 40), &mut held, &mut visible).is_none(), "not even for a new size");
  }

  #[test]
  fn the_gpu_canvas_is_let_go_when_a_shape_makes_the_frame_a_cpu_one_and_made_again_for_lines() {
    let Some(_) = gpu::context() else { return };
    let list = list();
    let (mut held, mut visible) = (None, false);
    list.set("line".into(), Gizmo::Line { points: vec![(5.0, 10.0), (35.0, 10.0)], brush: brush(&Color::red(), 9), drawn: 0 });
    assert!(matches!(list.next_frame((40, 20), &mut held, &mut visible), Some(RenderedFrame::Gpu(_))));
    assert!(held.is_some(), "a line is drawn on a GPU canvas");
    list.set("ring".into(), Gizmo::Outline(abra::prelude::Area::circle((20.0, 10.0), 8.0), brush(&Color::white(), 3)));
    assert!(matches!(list.next_frame((40, 20), &mut held, &mut visible), Some(RenderedFrame::Pixels(_))));
    assert!(held.is_none(), "an outline makes the frame on the CPU, so the canvas is let go");
    list.gizmos.lock().unwrap().retain(|(name, _)| name != "ring");
    list.rebuild.store(true, Ordering::SeqCst);
    assert!(matches!(list.next_frame((40, 20), &mut held, &mut visible), Some(RenderedFrame::Gpu(_))), "lines alone are on the GPU again");
    assert!(held.is_some());
  }

  #[test]
  fn lines_are_drawn_on_the_gpu_and_anything_else_on_the_cpu() {
    let list = list();
    let mut held = None;
    assert!(list.gpu_frame((40, 20), &mut held).is_none(), "nothing to draw");
    list.set("line".into(), Gizmo::Line { points: vec![(5.0, 10.0), (35.0, 10.0)], brush: brush(&Color::red(), 9), drawn: 0 });
    if gpu::context().is_some() {
      assert!(matches!(list.gpu_frame((40, 20), &mut held), Some(RenderedFrame::Gpu(_))));
    }
    list.set("ring".into(), Gizmo::Outline(abra::prelude::Area::circle((20.0, 10.0), 8.0), brush(&Color::white(), 3)));
    assert!(list.gpu_frame((40, 20), &mut held).is_none(), "an outline is made on the CPU");
  }

  /// A line that grows a point at a time, drawn by the GPU only where it is new, is the line drawn at once on the CPU, and it is
  /// the same after a replaced line makes the canvas start over.
  #[test]
  fn a_line_drawn_in_pieces_on_the_gpu_matches_the_cpu() {
    let Some(_) = gpu::context() else { return };
    let (width, height) = (96u32, 64u32);
    let component = Gizmos::new();
    let soft = abra_uniffi::Brush::new().with_size(12).with_hardness(0.3).with_opacity(0.8);
    let points = [(8.0, 50.0), (30.0, 12.0), (60.5, 40.0), (90.0, 8.0)];
    let mut held = None;
    let alpha_of = |frame: Option<RenderedFrame>| -> Vec<u8> {
      match frame.expect("a frame") {
        RenderedFrame::Gpu(frame) => frame.read_back().expect("read back").pixels.chunks(4).map(|pixel| pixel[3]).collect(),
        RenderedFrame::Pixels(frame) => frame.pixels.chunks(4).map(|pixel| pixel[3]).collect(),
      }
    };
    for (x, y) in points {
      component.add_point("line".into(), x, y, soft.clone());
      component.list.gpu_frame((width, height), &mut held);
    }
    let on_the_gpu = alpha_of(component.list.gpu_frame((width, height), &mut held));
    let on_the_cpu: Vec<u8> = component.list.frame(width, height).pixels.chunks(4).map(|pixel| pixel[3]).collect();
    let worst = on_the_gpu.iter().zip(&on_the_cpu).map(|(gpu, cpu)| gpu.abs_diff(*cpu)).max().unwrap();
    assert!(worst <= 8, "drawn in pieces, the GPU differs from the CPU by {worst}");
    assert!(on_the_gpu.iter().any(|&alpha| alpha > 200), "something was drawn");
    // Taking the line away clears the canvas.
    component.remove("line".into());
    component.add_point("other".into(), 48.0, 32.0, soft);
    let dot = alpha_of(component.list.gpu_frame((width, height), &mut held));
    assert!(dot[(32 * width + 48) as usize] > 150 && dot[(50 * width + 8) as usize] == 0, "only the new line is there");
  }

  /// How long a frame takes while a finger draws on a phone-sized view: one point added, then the frame made. Run with
  /// `cargo test -p alakazam-mobile --release timing -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_a_frame_while_drawing() {
    let Some(_) = gpu::context() else { return };
    let (width, height) = (1080u32, 2400u32);
    let component = Gizmos::new();
    let glow = abra_uniffi::Brush::new().with_size(42).with_hardness(0.0).with_opacity(0.5);
    let line = abra_uniffi::Brush::new().with_size(9).with_hardness(1.0);
    let mut held = None;
    let mut times = Vec::new();
    for step in 0..300 {
      let angle = step as f32 / 300.0 * std::f32::consts::TAU;
      let (x, y) = (540.0 + 400.0 * angle.cos(), 1200.0 + 400.0 * angle.sin());
      let started = std::time::Instant::now();
      component.add_point("glow".into(), x, y, glow.clone());
      component.add_point("line".into(), x, y, line.clone());
      let Some(RenderedFrame::Gpu(frame)) = component.list.gpu_frame((width, height), &mut held) else { panic!("not on the GPU") };
      // Wait for the GPU to finish, so the time is the drawing and not only the submitting.
      let _ = frame.read_back();
      times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let rest = &times[1..];
    println!(
      "a frame while drawing: {:.2} ms on average, {:.2} ms at the end, worst {:.2} ms (includes reading the picture back to wait)",
      rest.iter().sum::<f64>() / rest.len() as f64,
      rest[rest.len() - 20..].iter().sum::<f64>() / 20.0,
      rest.iter().cloned().fold(0.0, f64::max)
    );
  }

  #[test]
  fn with_no_gizmos_the_picture_is_clear() {
    assert!(list().frame(8, 4).pixels.iter().all(|&byte| byte == 0));
  }

  #[test]
  fn a_line_is_on_the_picture_and_clear_away_from_it() {
    let list = list();
    list.set("line".into(), Gizmo::Line { points: vec![(5.0, 10.0), (35.0, 10.0)], brush: brush(&Color::red(), 9), drawn: 0 });
    let frame = list.frame(40, 20);
    assert_eq!(pixel(&frame, 20, 10), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 20, 2)[3], 0);
  }

  #[test]
  fn an_outline_has_an_edge_and_an_empty_middle_and_a_fill_has_both() {
    let list = list();
    let ring = abra::prelude::Area::circle((20.0, 20.0), 15.0);
    list.set("ring".into(), Gizmo::Outline(ring.clone(), brush(&Color::white(), 3)));
    let frame = list.frame(40, 40);
    assert_eq!(pixel(&frame, 20, 20)[3], 0, "the middle of an outline is clear");
    assert!(pixel(&frame, 35, 20)[3] > 200, "its edge is drawn");
    list.set("ring".into(), Gizmo::Fill(ring, brush(&Color::white(), 3)));
    assert!(pixel(&list.frame(40, 40), 20, 20)[3] > 200, "a fill covers the middle");
  }

  #[test]
  fn a_name_that_is_set_again_keeps_its_place_and_the_newest_is_on_top() {
    let list = list();
    let across = |color: &Color| Gizmo::Line { points: vec![(5.0, 10.0), (35.0, 10.0)], brush: brush(color, 9), drawn: 0 };
    list.set("under".into(), across(&Color::red()));
    list.set("over".into(), across(&Color::blue()));
    assert_eq!(pixel(&list.frame(40, 20), 20, 10), [0, 0, 255, 255]);
    list.set("under".into(), across(&Color::green()));
    assert_eq!(list.gizmos.lock().unwrap()[0].0, "under");
    assert_eq!(pixel(&list.frame(40, 20), 20, 10), [0, 0, 255, 255], "re-setting the lower one does not lift it");
  }

  #[test]
  fn removing_a_gizmo_clears_its_pixels_and_a_change_is_a_new_version() {
    let component = Gizmos::new();
    let before = component.list.version.load(Ordering::SeqCst);
    component.add_point("lasso".into(), 5.0, 10.0, Arc::new(abra_uniffi::Brush::new()));
    component.add_point("lasso".into(), 30.0, 10.0, Arc::new(abra_uniffi::Brush::new()));
    assert_eq!(component.point_count("lasso".into()), 2);
    assert_ne!(component.list.version.load(Ordering::SeqCst), before);
    assert!(component.list.frame(40, 20).pixels.iter().any(|&byte| byte != 0));
    component.remove("lasso".into());
    assert_eq!(component.point_count("lasso".into()), 0);
    assert!(component.list.frame(40, 20).pixels.iter().all(|&byte| byte == 0));
  }
}
