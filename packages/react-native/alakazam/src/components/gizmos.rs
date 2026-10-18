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
}

impl Gizmo {
  /// The gizmo as a picture of `width` x `height` pixels: transparent everywhere except where it is.
  fn layer(&self, width: u32, height: u32) -> Pixels {
    match self {
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
  /// The renderer that makes the pictures, once there is one, so it can be woken when a gizmo changes.
  renderer: Mutex<Option<Offscreen>>,
}

impl GizmoList {
  /// Every gizmo over a clear picture of `width` x `height` pixels, the newest on top.
  fn frame(&self, width: u32, height: u32) -> Frame {
    let gizmos = self.gizmos.lock().unwrap();
    let mut picture = Pixels::new(width, height);
    for (_, gizmo) in gizmos.iter() {
      picture = blended(&picture, &gizmo.layer(width, height), BlendMode::Normal, 1.0);
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

  fn changed(&self) {
    self.version.fetch_add(1, Ordering::SeqCst);
    if let Some(renderer) = self.renderer.lock().unwrap().as_ref() {
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
    if gizmos.is_empty() || gizmos.iter().any(|(_, gizmo)| !matches!(gizmo, Gizmo::Line { .. })) {
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
    for (_, gizmo) in gizmos.iter_mut() {
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
    let renderer = Offscreen::tracking(size, version, || false, move |size| {
      if size.0 == 0 || size.1 == 0 {
        return None;
      }
      if let Some(frame) = list.gpu_frame(*size, &mut held) {
        return Some(frame);
      }
      Some(RenderedFrame::Pixels(Arc::new(list.frame(size.0, size.1))))
    });
    *self.0.renderer.lock().unwrap() = Some(renderer.clone());
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
    let list = Arc::new(GizmoList { gizmos: Mutex::new(Vec::new()), version: AtomicU64::new(0), rebuild: AtomicBool::new(true), renderer: Mutex::new(None) });
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
    GizmoList { gizmos: Mutex::new(Vec::new()), version: AtomicU64::new(0), rebuild: AtomicBool::new(true), renderer: Mutex::new(None) }
  }

  fn brush(color: &Color, size: u32) -> Brush<'static> {
    Brush::new().with_size(size).with_hardness(1.0).with_color(color)
  }

  fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
    let at = (y * frame.width as usize + x) * 4;
    [frame.pixels[at], frame.pixels[at + 1], frame.pixels[at + 2], frame.pixels[at + 3]]
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
