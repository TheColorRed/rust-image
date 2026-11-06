//! Handles over the photo: points that a finger grabs and moves, such as the two ends of a gradient, the four corners of a perspective correction or the
//! two points of a line to straighten. What changes from one tool to the next is how many handles there are, how each looks and how they are joined;
//! grabbing, moving, staying on the photo as it is zoomed, panned and turned, and drawing are the same, so they are done once, here.

use std::sync::{Arc, Mutex};

use abra::abra_core::{Area, Fill, Image as Pixels, LineCap, Path};
use abra::drawing::prelude::fill;
use vessel::prelude::*;

use crate::components::{camera::Camera, image_preview::ImagePreview, layer};

/// The radius of a handle's dot, in dp.
const HANDLE_RADIUS: f32 = 12.0;
/// How close a finger has to land to a handle to take it, in dp.
const GRAB: f32 = 26.0;
/// The dark that goes under the white of a handle and a line, so they show on light and dark parts of the photo alike.
const SHADOW: [u8; 4] = [0, 0, 0, 115];
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// How a handle looks, so which is which can be told at a glance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum HandleShape {
  /// A solid white dot.
  Dot,
  /// A white ring with the photo showing through.
  Ring,
  /// A white square.
  Square,
}

/// How the handles are joined, in the order they were given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Joined {
  /// Not at all.
  Nothing,
  /// By a line from the first to the last.
  Line,
  /// By a closed outline: a line from the first to the last, and from the last back to the first.
  Outline,
}

/// How far from the middle of a handle it reaches, in pixels: the dot, the dark round it and a pixel for the smoothing at the edge.
fn reach(p_density: f32) -> f32 {
  ((HANDLE_RADIUS + 3.0) * p_density).ceil() + 1.0
}

/// A handle a finger has hold of.
#[derive(Clone, Copy)]
struct Held {
  index: usize,
  /// Where the handle was in the view, and where the finger was, when it was taken: the handle moves by how far the finger has gone since, so it
  /// does not jump to the finger.
  handle: (f32, f32),
  finger: (f32, f32),
}

/// The handles, where they are on the photo, and the children that show them.
struct Parts {
  /// Where each handle is on the photo, as shares of the photo (0 to 1 across and down), so they mean the same at any size of the photo.
  points: BehaviorSubject<Vec<(f32, f32)>>,
  /// Where the view looks at the photo, once it is known.
  camera: BehaviorSubject<Option<Camera>>,
  /// The size of the component, which is the preview's size and the room round it on each side (`overscan`).
  size: BehaviorSubject<(u32, u32)>,
  /// The room on each side, in pixels, that lets a handle at the edge of the preview be whole.
  overscan: u32,
  density: f32,
  /// Whether a handle is held on the photo, so it cannot be moved off it.
  inside: bool,
  /// The child that shows each handle, in the order of the points.
  handles: Vec<Component>,
  held: Mutex<Option<Held>>,
  /// Held while the handles are put in place, so two changes at once cannot leave a handle where the older one put it. Nothing else takes it, and
  /// putting a handle in place never comes back here, so it cannot be waited for in a circle.
  arranging: Mutex<()>,
}

impl Parts {
  /// The size of the preview, which is what the camera is looking through.
  fn view_size(&self) -> (u32, u32) {
    let size = self.size.peek();
    (size.0.saturating_sub(self.overscan * 2), size.1.saturating_sub(self.overscan * 2))
  }

  /// A place on the photo, as shares of it, as a place in the preview in pixels.
  fn in_view(&self, p_camera: &Camera, p_point: (f32, f32)) -> (f32, f32) {
    p_camera.to_view((p_point.0 * p_camera.photo.0, p_point.1 * p_camera.photo.1), self.view_size())
  }

  /// Puts each handle's child where its point is, or takes it away when the point is out of sight.
  fn arrange(&self) {
    let _arranging = self.arranging.lock().unwrap();
    let (camera, points) = (self.camera.peek(), self.points.peek());
    let reach = reach(self.density);
    let side = (reach * 2.0) as u32;
    for (index, handle) in self.handles.iter().enumerate() {
      let spot = camera.zip(points.get(index)).map(|(camera, point)| {
        let (x, y) = self.in_view(&camera, *point);
        (x + self.overscan as f32 - reach, y + self.overscan as f32 - reach)
      });
      match spot {
        Some((x, y)) if x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0 => handle.place(x.round() as u32, y.round() as u32, side, side),
        // Not in sight, or further out than the room round the preview: the handle is not shown.
        _ => handle.place(0, 0, 0, 0),
      }
    }
  }

  /// The handle nearest to a place in the preview, when it is near enough to be taken.
  fn nearest(&self, p_at: (f32, f32)) -> Option<usize> {
    let camera = self.camera.peek()?;
    self
      .points
      .peek()
      .iter()
      .enumerate()
      .map(|(index, point)| {
        let place = self.in_view(&camera, *point);
        (index, (place.0 - p_at.0).hypot(place.1 - p_at.1))
      })
      .filter(|(_, distance)| *distance <= GRAB * self.density)
      .min_by(|a, b| a.1.total_cmp(&b.1))
      .map(|(index, _)| index)
  }

  /// Takes the handle at a place in the preview, if there is one there.
  fn grab(&self, p_at: (f32, f32)) -> bool {
    let (Some(camera), Some(index)) = (self.camera.peek(), self.nearest(p_at)) else { return false };
    let handle = self.in_view(&camera, self.points.peek()[index]);
    *self.held.lock().unwrap() = Some(Held { index, handle, finger: p_at });
    true
  }

  /// Moves the handle that is held by how far the finger has gone since it was taken.
  fn drag(&self, p_to: (f32, f32)) {
    let held = *self.held.lock().unwrap();
    let (Some(held), Some(camera)) = (held, self.camera.peek()) else { return };
    let moved = (held.handle.0 + p_to.0 - held.finger.0, held.handle.1 + p_to.1 - held.finger.1);
    let photo = camera.to_photo(moved, self.view_size());
    let mut share = (photo.0 / camera.photo.0, photo.1 / camera.photo.1);
    if self.inside {
      share = (share.0.clamp(0.0, 1.0), share.1.clamp(0.0, 1.0));
    }
    let mut points = self.points.peek();
    if let Some(point) = points.get_mut(held.index) {
      *point = share;
      self.points.next(points);
      self.arrange();
    }
  }
}

/// A line through `p_points`, `p_width` wide, with round ends.
fn stroke(p_image: &mut Pixels, p_points: &[(f32, f32)], p_width: f32, p_color: [u8; 4]) {
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

/// The picture of one handle in the middle of a square of `p_size` pixels.
fn handle_picture(p_shape: HandleShape, p_density: f32, p_size: (u32, u32)) -> Pixels {
  let mut image = Pixels::new(p_size.0, p_size.1);
  let middle = (p_size.0 as f32 / 2.0, p_size.1 as f32 / 2.0);
  let (radius, density) = (HANDLE_RADIUS * p_density, p_density);
  match p_shape {
    HandleShape::Dot => {
      fill(Area::circle(middle, radius + 1.5 * density), Fill::solid(SHADOW)).apply(&mut image);
      fill(Area::circle(middle, radius), Fill::solid(WHITE)).apply(&mut image);
    }
    HandleShape::Ring => {
      fill(Area::circle(middle, radius), Fill::solid([255, 255, 255, 38])).apply(&mut image);
      fill(Area::circle(middle, radius + density).stroke(2.0 * density), Fill::solid(SHADOW)).apply(&mut image);
      fill(Area::circle(middle, radius - 1.5 * density).stroke(3.0 * density), Fill::solid(WHITE)).apply(&mut image);
    }
    HandleShape::Square => {
      let (shadow, body) = (radius * 0.9 + 1.5 * density, radius * 0.9);
      fill(Area::rect((middle.0 - shadow, middle.1 - shadow), (shadow * 2.0, shadow * 2.0)), Fill::solid(SHADOW)).apply(&mut image);
      fill(Area::rect((middle.0 - body, middle.1 - body), (body * 2.0, body * 2.0)), Fill::solid(WHITE)).apply(&mut image);
    }
  }
  image
}

/// Handles over the photo: points that a finger grabs and moves, joined by a line or an outline if the tool wants, and kept on the same place of the
/// photo as it is zoomed, panned and turned. A tool says how many handles there are, how each looks and how they are joined; this does the rest, so the
/// gradient's two ends, a perspective correction's four corners and a straightening line's two points are all the same component.
///
/// The photo's touches are the preview's, so the preview is where a finger lands first. The caller asks `grab` whether a touch is on a handle and gives
/// the touch to the handles (`drag_to`, `end_drag`) when it is, and to the preview when it is not. The caller reads `points` as they move, to show the
/// tool's result, and when the finger lifts, to save it.
///
/// It is made of Vessel children: a line or outline over the whole view, drawn again when a point or the camera moves, and a small child for each
/// handle that is drawn once and only moved. Show it on a view that reaches `overscan` pixels past the preview on every side, so a handle at the preview's
/// edge is not cut in half; every place the caller gives or reads is in the preview's own pixels.
#[derive(uniffi::Object, Component)]
pub struct Handles {
  root: Container,
  parts: Arc<Parts>,
  /// What keeps the handles on the same part of the photo as the view they follow (see [`follow`](Self::follow)).
  following: Mutex<Option<Subscription>>,
}

#[uniffi::export]
impl Handles {
  /// Handles at `points`, which are `x, y` pairs in one list as shares of the photo (0 at the left or top, 1 at the right or bottom). `shapes` has the
  /// look of each handle, in the same order. `joined` says how they are joined. With `inside_photo` a handle cannot be moved off the photo.
  /// `density` converts the handles' dp measurements to pixels.
  ///
  /// # Panics
  /// When there are no points, or `points` and `shapes` do not hold the same number of handles.
  #[uniffi::constructor]
  pub fn new(points: Vec<f64>, shapes: Vec<HandleShape>, joined: Joined, inside_photo: bool, density: f64) -> Arc<Self> {
    assert!(!shapes.is_empty() && points.len() == shapes.len() * 2, "each handle has a place (x, y) and a shape");
    let density = (density as f32).max(0.1);
    let overscan = reach(density) as u32;
    let points = BehaviorSubject::new(shares(&points));
    let camera = BehaviorSubject::new(None::<Camera>);

    let root = Container::new();
    if joined != Joined::Nothing {
      // The line or outline: it depends on the points and the camera, and is drawn again when either moves.
      let line = layer("handles-line", {
        let (points, camera) = (points.clone(), camera.clone());
        move |size| {
          let (points, camera) = (points.value(), camera.value());
          let mut image = Pixels::new(size.0, size.1);
          let Some(camera) = camera else { return image };
          let view = (size.0.saturating_sub(overscan * 2), size.1.saturating_sub(overscan * 2));
          let mut shape: Vec<(f32, f32)> = points
            .iter()
            .map(|point| {
              let (x, y) = camera.to_view((point.0 * camera.photo.0, point.1 * camera.photo.1), view);
              (x + overscan as f32, y + overscan as f32)
            })
            .collect();
          if joined == Joined::Outline && !shape.is_empty() {
            shape.push(shape[0]);
          }
          if shape.len() >= 2 {
            stroke(&mut image, &shape, 4.0 * density, SHADOW);
            stroke(&mut image, &shape, 2.0 * density, WHITE);
          }
          image
        }
      });
      root.add(&line);
    }
    let handles: Vec<Component> = shapes.iter().map(|shape| {
      let shape = *shape;
      layer("handles-handle", move |size| handle_picture(shape, density, size))
    }).collect();
    for handle in &handles {
      root.add(handle);
    }

    let parts = Arc::new(Parts {
      points,
      camera,
      size: root.size().clone(),
      overscan,
      density,
      inside: inside_photo,
      handles,
      held: Mutex::new(None),
      arranging: Mutex::new(()),
    });
    parts.arrange();
    root.keep(root.size().subscribe({
      let parts = Arc::clone(&parts);
      move |_| parts.arrange()
    }));
    Arc::new(Self { root, parts, following: Mutex::new(None) })
  }

  /// How many pixels past the preview on every side the view that shows this has to reach, so a handle at the preview's edge is whole.
  pub fn overscan(&self) -> u32 {
    self.parts.overscan
  }

  /// Makes the handles follow `preview`: they stay on the same places of the photo as it is zoomed, panned and turned, on every frame of a glide too.
  /// Following another view stops following the first.
  pub fn follow(&self, preview: Arc<ImagePreview>) {
    let parts = Arc::clone(&self.parts);
    let following = preview.follow_camera(move |camera| {
      parts.camera.next(Some(camera));
      parts.arrange();
    });
    // The old subscription is let go outside the lock.
    let before = self.following.lock().unwrap().replace(following);
    drop(before);
  }

  /// Where the handles are now, as `x, y` pairs in one list, as shares of the photo, in the order they were given.
  pub fn points(&self) -> Vec<f64> {
    self.parts.points.peek().iter().flat_map(|(x, y)| [*x as f64, *y as f64]).collect()
  }

  /// Puts the handles at `points`, as when a tool's saved values come back (undo, a reset). Does nothing to a handle that is held.
  ///
  /// # Panics
  /// When there are not as many points as there are handles.
  pub fn set_points(&self, points: Vec<f64>) {
    assert_eq!(points.len(), self.parts.handles.len() * 2, "a place (x, y) for each handle");
    if self.parts.held.lock().unwrap().is_some() {
      return;
    }
    self.parts.points.next(shares(&points));
    self.parts.arrange();
  }

  /// The place of the handle at a place in the preview (in pixels from its top left corner) in the order they were given, or -1 when none is near
  /// enough to be taken. Nothing is taken.
  pub fn handle_at(&self, x: f32, y: f32) -> i32 {
    self.parts.nearest((x, y)).map_or(-1, |index| index as i32)
  }

  /// Takes the handle at a place in the preview, if there is one near enough, and says whether it did. Moving the finger then moves the handle
  /// (`drag_to`) until `end_drag`.
  pub fn grab(&self, x: f32, y: f32) -> bool {
    self.parts.grab((x, y))
  }

  /// Which handle is held, in the order they were given, or -1 when none is.
  pub fn held(&self) -> i32 {
    self.parts.held.lock().unwrap().map_or(-1, |held| held.index as i32)
  }

  /// Moves the held handle by how far the finger, now at a place in the preview, has gone since the handle was taken. Does nothing when none is held.
  pub fn drag_to(&self, x: f32, y: f32) {
    self.parts.drag((x, y));
  }

  /// Lets the held handle go.
  pub fn end_drag(&self) {
    *self.parts.held.lock().unwrap() = None;
  }
}

/// `x, y` pairs from one list.
fn shares(p_list: &[f64]) -> Vec<(f32, f32)> {
  p_list.chunks_exact(2).map(|pair| (pair[0] as f32, pair[1] as f32)).collect()
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::components::pixel;

  /// A view of a 100 x 50 photo, whose size is set as the first picture would set it.
  fn preview() -> Arc<ImagePreview> {
    let photo = abra_uniffi::Image::from_core(abra::abra_core::Image::new_from_color(100, 50, abra::abra_core::Color::from_rgba(90, 90, 90, 255)));
    let preview = ImagePreview::new(Arc::new(photo));
    preview.source.photo.next((100, 50));
    preview
  }

  /// Two handles on a line, at the left quarter and the right quarter of the photo's middle, following a preview that is 200 x 100 pixels.
  fn handles() -> (Arc<Handles>, Arc<ImagePreview>) {
    let handles = Handles::new(vec![0.25, 0.5, 0.75, 0.5], vec![HandleShape::Dot, HandleShape::Ring], Joined::Line, true, 1.0);
    let overscan = handles.overscan();
    assert_eq!(overscan, 16);
    handles.size().next((200 + overscan * 2, 100 + overscan * 2));
    let preview = preview();
    handles.follow(Arc::clone(&preview));
    (handles, preview)
  }

  fn render(p_handles: &Handles) -> Frame {
    let mut component = p_handles.component().clone();
    component.render(Duration::ZERO)
  }

  fn close(p_a: &[f64], p_b: &[f64]) -> bool {
    p_a.len() == p_b.len() && p_a.iter().zip(p_b).all(|(a, b)| (a - b).abs() < 0.002)
  }

  #[test]
  fn handles_are_drawn_where_their_points_are_on_the_photo_and_joined_by_a_line() {
    let (handles, _preview) = handles();
    let frame = render(&handles);
    // The first handle is at (0.25, 0.5) of the photo: (50, 50) in the preview, and the preview starts 16 pixels in.
    assert_eq!(pixel(&frame, 66, 66), WHITE, "the solid dot is white in the middle");
    // The second is a ring: open inside it (off the line that runs to its middle), white on its edge, and the line runs between the two.
    assert!(pixel(&frame, 166, 60)[3] < 100, "the ring is open inside: {:?}", pixel(&frame, 166, 60));
    assert_eq!(pixel(&frame, 166, 66 - 11)[..3], WHITE[..3], "and white on its edge");
    assert_eq!(pixel(&frame, 116, 66), WHITE, "the line is white between them");
    assert_eq!(pixel(&frame, 116, 20)[3], 0, "and clear away from them");
  }

  #[test]
  fn a_handle_is_taken_near_it_and_moves_by_how_far_the_finger_goes() {
    let (handles, _preview) = handles();
    assert_eq!(handles.handle_at(52.0, 52.0), 0);
    assert_eq!(handles.handle_at(150.0, 50.0), 1);
    assert_eq!(handles.handle_at(100.0, 10.0), -1, "nothing is there");
    assert!(!handles.grab(100.0, 10.0));
    assert_eq!(handles.held(), -1);
    assert!(handles.grab(54.0, 50.0));
    assert_eq!(handles.held(), 0);
    // The finger goes 20 right and 10 down from where it landed: the handle goes the same way, from (50, 50) to (70, 60) of the 200 x 100 preview.
    handles.drag_to(74.0, 60.0);
    assert!(close(&handles.points(), &[0.35, 0.6, 0.75, 0.5]), "{:?}", handles.points());
    handles.end_drag();
    assert_eq!(handles.held(), -1);
    handles.drag_to(10.0, 10.0);
    assert!(close(&handles.points(), &[0.35, 0.6, 0.75, 0.5]), "a handle that is let go does not follow");
  }

  #[test]
  fn a_handle_is_held_on_the_photo() {
    let (handles, _preview) = handles();
    assert!(handles.grab(50.0, 50.0));
    handles.drag_to(900.0, -900.0);
    assert!(close(&handles.points()[..2], &[1.0, 0.0]), "{:?}", handles.points());
  }

  #[test]
  fn the_handles_stay_on_the_photo_as_the_view_is_zoomed_and_turned() {
    let (handles, preview) = handles();
    // The view shows the right half of the photo: the second handle, at 0.75 across, is in the middle of the preview, at (100, 50).
    preview.set_viewport(0.5, 0.0, 0.5, 1.0);
    assert_eq!(handles.handle_at(100.0, 50.0), 1);
    assert_eq!(handles.handle_at(50.0, 50.0), -1, "the first is off to the left of what is shown");
    assert!(handles.grab(100.0, 50.0));
    handles.drag_to(120.0, 50.0);
    // 20 pixels of a view that shows half the photo's width in 200 pixels is a twentieth of the photo.
    assert!(close(&handles.points()[2..], &[0.8, 0.5]), "{:?}", handles.points());
    handles.end_drag();
    // Turned a quarter: a handle under the finger is still under it after a drag that does not move.
    preview.set_viewport(0.0, 0.0, 1.0, 1.0);
    preview.set_rotation(std::f32::consts::FRAC_PI_2);
    let before = handles.points();
    let place = |points: &[f64]| {
      // Turned a quarter clockwise about the middle of the preview (100, 50): the share (x, y) of a 100 x 50 photo seen at 200 x 100.
      let (across, down) = (points[0] * 200.0 - 100.0, points[1] * 100.0 - 50.0);
      (100.0 - down, 50.0 + across)
    };
    let (x, y) = place(&before);
    assert_eq!(handles.handle_at(x as f32, y as f32), 0, "the handle is where the turned photo has it ({x}, {y})");
    assert!(handles.grab(x as f32, y as f32));
    handles.drag_to(x as f32, y as f32);
    assert!(close(&handles.points(), &before), "a drag that does not move changes nothing: {:?} and {before:?}", handles.points());
  }

  #[test]
  fn the_app_can_set_the_points_but_not_under_a_finger() {
    let (handles, _preview) = handles();
    handles.set_points(vec![0.1, 0.2, 0.3, 0.4]);
    assert!(close(&handles.points(), &[0.1, 0.2, 0.3, 0.4]));
    assert!(handles.grab(20.0, 20.0), "the first handle is now at (20, 20)");
    handles.set_points(vec![0.9, 0.9, 0.9, 0.9]);
    assert!(close(&handles.points(), &[0.1, 0.2, 0.3, 0.4]), "a handle that is held is not moved from outside");
    handles.end_drag();
    handles.set_points(vec![0.9, 0.9, 0.9, 0.9]);
    assert!(close(&handles.points(), &[0.9, 0.9, 0.9, 0.9]));
  }

  #[test]
  fn a_handle_at_the_edge_of_the_preview_is_whole_and_one_far_outside_is_not_shown() {
    let handles = Handles::new(vec![0.0, 0.0, 1.0, 1.0], vec![HandleShape::Dot, HandleShape::Square], Joined::Nothing, false, 3.0);
    let overscan = handles.overscan();
    handles.size().next((200 + overscan * 2, 100 + overscan * 2));
    handles.follow(preview());
    for (index, handle) in handles.parts.handles.iter().enumerate() {
      let mut child = handle.clone();
      let side = overscan * 2;
      child.size().next((side, side));
      let frame = child.render(Duration::ZERO);
      assert_eq!(frame.width, side, "handle {index} has the room to be whole");
      let edge: Vec<[u8; 4]> = (0..side).flat_map(|at| [(0, at), (side - 1, at), (at, 0), (at, side - 1)]).map(|(x, y)| pixel(&frame, x, y)).collect();
      assert!(edge.iter().all(|pixel| pixel[3] == 0), "handle {index} reaches the edge of its child");
    }
    // The first is at the preview's top left corner, which is `overscan` pixels in from the component's.
    let frame = render(&handles);
    assert_eq!(pixel(&frame, overscan, overscan), WHITE);
    // Moved off the photo and out of the room round the preview, a handle is gone and the others are not.
    handles.set_points(vec![-5.0, 0.5, 1.0, 1.0]);
    let frame = render(&handles);
    assert!(frame.pixels.chunks(4).filter(|pixel| pixel[3] > 0).count() > 0, "the other handle is still shown");
    assert_eq!(pixel(&frame, 0, overscan + 50)[3], 0);
  }

  /// Grabbing and moving on one thread while another draws and the view moves must not leave any of them waiting for another. A hang fails the
  /// test at the timeout.
  #[test]
  fn moving_handles_while_the_view_moves_and_another_thread_draws_does_not_wait_forever() {
    let (handles, preview) = handles();
    let (done, finished) = std::sync::mpsc::channel();
    let drawing = {
      let (handles, done) = (Arc::clone(&handles), done.clone());
      std::thread::spawn(move || {
        for _ in 0..200 {
          render(&handles);
        }
        done.send(()).unwrap();
      })
    };
    let viewing = {
      let (preview, done) = (Arc::clone(&preview), done.clone());
      std::thread::spawn(move || {
        for step in 0..300u32 {
          preview.set_viewport(step as f32 / 900.0, 0.0, 0.6, 1.0);
          preview.set_rotation(step as f32 / 200.0);
        }
        done.send(()).unwrap();
      })
    };
    let dragging = {
      let handles = Arc::clone(&handles);
      std::thread::spawn(move || {
        for step in 0..300u32 {
          if step % 50 == 0 {
            handles.end_drag();
            let _ = handles.grab(100.0, 50.0);
          }
          handles.drag_to(100.0 + (step % 60) as f32, 50.0 + (step % 30) as f32);
          handles.set_points(vec![0.3, 0.3, 0.6, 0.6]);
        }
        done.send(()).unwrap();
      })
    };
    for _ in 0..3 {
      finished.recv_timeout(Duration::from_secs(30)).expect("the handles, the view and the drawing are waiting for each other");
    }
    drawing.join().unwrap();
    viewing.join().unwrap();
    dragging.join().unwrap();
  }

  #[test]
  fn dropping_the_handles_frees_them_and_what_follows_them() {
    let (handles, preview) = handles();
    let weak = handles.component().downgrade();
    render(&handles);
    assert!(handles.grab(50.0, 50.0));
    drop(handles);
    assert!(weak.upgrade().is_none(), "the handles are still alive after they were dropped: something in them holds itself");
    // The view they followed goes on without them.
    preview.set_viewport(0.1, 0.0, 0.5, 1.0);
  }

  /// How long a frame takes while a handle is dragged on a phone-sized preview. Run with
  /// `cargo test -p alakazam-mobile --lib components::handles::tests::timing -- --ignored --nocapture`.
  #[test]
  #[ignore]
  fn timing_of_a_frame_while_dragging_a_handle() {
    let handles = Handles::new(vec![0.25, 0.5, 0.75, 0.5], vec![HandleShape::Dot, HandleShape::Ring], Joined::Line, true, 3.0);
    let overscan = handles.overscan();
    handles.size().next((1000 + overscan * 2, 1400 + overscan * 2));
    let preview = preview();
    handles.follow(Arc::clone(&preview));
    render(&handles);
    assert!(handles.grab(250.0, 700.0));
    let mut frames = Vec::new();
    for step in 0..300u32 {
      let started = std::time::Instant::now();
      handles.drag_to(250.0 + (step % 100) as f32, 700.0 + (step % 80) as f32);
      render(&handles);
      frames.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    println!(
      "moving a handle and drawing the frame: {:.3} ms on average, worst {:.3} ms",
      frames.iter().sum::<f64>() / frames.len() as f64,
      frames.iter().cloned().fold(0.0, f64::max)
    );
  }
}
