use vessel::prelude::Viewport;

/// Where a view looks at the photo, so something placed on the photo (a gizmo, a handle) is drawn where that part of the photo is on the screen.
/// Set with the same numbers as the photo's own view, so the two always agree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Camera {
  /// The size of the photo in pixels.
  pub(crate) photo: (f32, f32),
  /// The part of the photo the view shows, as shares of the photo. It can reach past the photo's edges.
  pub(crate) viewport: Viewport,
  /// How far the photo is turned, clockwise, in radians, about the middle of the view.
  pub(crate) angle: f32,
}

impl Camera {
  /// The camera for a photo of `p_photo` pixels seen through `p_viewport` turned `p_angle` radians, or `None` while the photo has no size or the
  /// viewport shows nothing.
  pub(crate) fn of(p_viewport: Viewport, p_angle: f32, p_photo: (u32, u32)) -> Option<Self> {
    (p_photo.0 > 0 && p_photo.1 > 0 && p_viewport.width > 0.0 && p_viewport.height > 0.0)
      .then(|| Self { photo: (p_photo.0 as f32, p_photo.1 as f32), viewport: p_viewport, angle: p_angle })
  }

  /// A point of the photo, in photo pixels, as a point of a view of `p_size` pixels.
  pub(crate) fn to_view(&self, p_point: (f32, f32), p_size: (u32, u32)) -> (f32, f32) {
    self.viewport.to_view(p_point, self.photo, (p_size.0 as f32, p_size.1 as f32), self.angle)
  }

  /// A point of a view of `p_size` pixels as a point of the photo, in photo pixels: the opposite of [`to_view`](Self::to_view).
  pub(crate) fn to_photo(&self, p_point: (f32, f32), p_size: (u32, u32)) -> (f32, f32) {
    self.viewport.to_source(p_point, self.photo, (p_size.0 as f32, p_size.1 as f32), self.angle)
  }
}
