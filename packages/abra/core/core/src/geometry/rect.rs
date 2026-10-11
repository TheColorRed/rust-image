//! Axis-aligned rectangles and aspect-ratio fitting.
//!
//! A [`Rect`] is the upright box used for bounds, clipping, and coordinate systems (the role an SVG `viewBox`
//! plays). [`AspectRatio`] describes how one rectangle is scaled into another of a different shape.

use crate::{FromF32, IntoNumber, PointF, Size};

/// An upright rectangle with its top-left corner at `(x, y)`.
///
/// A rectangle with no width or height is empty; [`Rect::intersect`] returns one when two rectangles do not
/// overlap.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
  /// The left edge.
  pub x: f32,
  /// The top edge.
  pub y: f32,
  /// The width. Never negative.
  pub width: f32,
  /// The height. Never negative.
  pub height: f32,
}

impl Rect {
  /// Creates a rectangle from its top-left corner and size.
  /// - `p_origin`: The top-left corner.
  /// - `p_size`: The width and height. Negative values are treated as zero.
  pub fn new(p_origin: impl Into<PointF>, p_size: impl Into<Size>) -> Rect {
    let (origin, size) = (p_origin.into(), p_size.into());
    Rect {
      x: origin.x,
      y: origin.y,
      width: size.width.max(0.0),
      height: size.height.max(0.0),
    }
  }

  /// Creates a rectangle from its four edges. Edges given in the wrong order produce an empty rectangle.
  pub fn from_edges(
    p_left: impl IntoNumber, p_top: impl IntoNumber, p_right: impl IntoNumber, p_bottom: impl IntoNumber,
  ) -> Rect {
    let (left, top) = (p_left.into::<f32>(), p_top.into::<f32>());
    let (right, bottom) = (p_right.into::<f32>(), p_bottom.into::<f32>());
    Rect::new((left, top), (right - left, bottom - top))
  }

  /// The smallest rectangle containing every point, or an empty rectangle at the origin when there are none.
  pub fn from_points(p_points: impl IntoIterator<Item = impl Into<PointF>>) -> Rect {
    let mut points = p_points.into_iter().map(Into::into);
    let Some(first) = points.next() else {
      return Rect::default();
    };
    let (min, max) = points.fold((first, first), |(min, max), point: PointF| {
      (PointF::new(min.x.min(point.x), min.y.min(point.y)), PointF::new(max.x.max(point.x), max.y.max(point.y)))
    });
    Rect::from_edges(min.x, min.y, max.x, max.y)
  }

  /// The left edge.
  pub fn left(&self) -> f32 {
    self.x
  }
  /// The top edge.
  pub fn top(&self) -> f32 {
    self.y
  }
  /// The right edge.
  pub fn right(&self) -> f32 {
    self.x + self.width
  }
  /// The bottom edge.
  pub fn bottom(&self) -> f32 {
    self.y + self.height
  }
  /// The top-left corner.
  pub fn origin(&self) -> PointF {
    PointF::new(self.x, self.y)
  }
  /// The width and height.
  pub fn size(&self) -> Size {
    Size::new(self.width, self.height)
  }
  /// The center point.
  pub fn center(&self) -> PointF {
    PointF::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
  }

  /// The edges as `(left, top, right, bottom)`, converted to `S` (integers are rounded).
  ///
  /// ```ignore
  /// let (min_x, min_y, max_x, max_y) = area.bounds().edges::<i32>();
  /// ```
  pub fn edges<S: FromF32>(&self) -> (S, S, S, S) {
    (S::from_f32(self.left()), S::from_f32(self.top()), S::from_f32(self.right()), S::from_f32(self.bottom()))
  }

  /// The corners in the order top-left, top-right, bottom-right, bottom-left.
  pub fn corners(&self) -> [PointF; 4] {
    let (left, top, right, bottom) = self.edges::<f32>();
    [
      PointF::new(left, top),
      PointF::new(right, top),
      PointF::new(right, bottom),
      PointF::new(left, bottom),
    ]
  }

  /// Whether the rectangle has no area.
  pub fn is_empty(&self) -> bool {
    !(self.width > 0.0 && self.height > 0.0)
  }

  /// Whether `p_point` is inside the rectangle. The left and top edges are inside, the right and bottom are not.
  pub fn contains(&self, p_point: impl Into<PointF>) -> bool {
    let point = p_point.into();
    point.x >= self.left() && point.x < self.right() && point.y >= self.top() && point.y < self.bottom()
  }

  /// The overlap of the two rectangles, or an empty rectangle when they do not overlap.
  ///
  /// ```ignore
  /// let visible = area.bounds().intersect(Rect::new((0, 0), image.size()));
  /// ```
  pub fn intersect(&self, p_other: impl Into<Rect>) -> Rect {
    let other = p_other.into();
    let (left, top) = (self.left().max(other.left()), self.top().max(other.top()));
    let (right, bottom) = (self.right().min(other.right()), self.bottom().min(other.bottom()));
    if right <= left || bottom <= top {
      return Rect::new((left, top), (0, 0));
    }
    Rect::from_edges(left, top, right, bottom)
  }

  /// Maps a point from this rectangle's coordinate system into a viewport of `p_viewport` size, the way an SVG
  /// `viewBox` maps into its element.
  /// - `p_point`: The point in this rectangle's coordinates.
  /// - `p_viewport`: The size of the target viewport, whose top-left corner is the origin.
  /// - `p_aspect_ratio`: How to scale when the shapes differ.
  pub fn map_point(&self, p_point: PointF, p_viewport: Size, p_aspect_ratio: AspectRatio) -> PointF {
    let (scale_x, scale_y) = (p_viewport.width / self.width, p_viewport.height / self.height);
    let (scale_x, scale_y) = match p_aspect_ratio.mode {
      PreserveAspectRatio::None => (scale_x, scale_y),
      PreserveAspectRatio::Meet => (scale_x.min(scale_y), scale_x.min(scale_y)),
      PreserveAspectRatio::Slice => (scale_x.max(scale_y), scale_x.max(scale_y)),
    };
    let offset = |alignment: Alignment, viewport: f32, scaled: f32| match alignment {
      Alignment::Min => 0.0,
      Alignment::Mid => (viewport - scaled) * 0.5,
      Alignment::Max => viewport - scaled,
    };
    let offset_x = offset(p_aspect_ratio.align_x, p_viewport.width, self.width * scale_x);
    let offset_y = offset(p_aspect_ratio.align_y, p_viewport.height, self.height * scale_y);
    PointF::new((p_point.x - self.x) * scale_x + offset_x, (p_point.y - self.y) * scale_y + offset_y)
  }
}

impl From<Size> for Rect {
  /// A rectangle of the given size at the origin.
  fn from(p_size: Size) -> Self {
    Rect::new((0, 0), p_size)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
/// Defines how content should be scaled when the source and target rectangles have different aspect ratios.
pub enum PreserveAspectRatio {
  /// Do not preserve aspect ratio, stretch to fill viewport.
  None,
  /// Preserve aspect ratio, scale to fit within viewport (may leave empty space). Default.
  #[default]
  Meet,
  /// Preserve aspect ratio, scale to cover entire viewport (may crop content).
  Slice,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
/// Alignment options when preserving aspect ratio.
pub enum Alignment {
  /// Align to minimum (left/top).
  Min,
  /// Align to center. Default.
  #[default]
  Mid,
  /// Align to maximum (right/bottom).
  Max,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Complete aspect ratio preservation specification.
pub struct AspectRatio {
  /// Scaling mode.
  pub mode: PreserveAspectRatio,
  /// Horizontal alignment.
  pub align_x: Alignment,
  /// Vertical alignment.
  pub align_y: Alignment,
}

impl AspectRatio {
  /// Creates a new aspect ratio specification.
  pub fn new(p_mode: PreserveAspectRatio, p_align_x: Alignment, p_align_y: Alignment) -> AspectRatio {
    AspectRatio {
      mode: p_mode,
      align_x: p_align_x,
      align_y: p_align_y,
    }
  }

  /// No aspect ratio preservation (stretch to fill).
  pub fn none() -> AspectRatio {
    AspectRatio::new(PreserveAspectRatio::None, Alignment::Mid, Alignment::Mid)
  }

  /// Preserve aspect ratio, fit within viewport, centered.
  pub fn meet() -> AspectRatio {
    AspectRatio::new(PreserveAspectRatio::Meet, Alignment::Mid, Alignment::Mid)
  }

  /// Preserve aspect ratio, cover entire viewport, centered.
  pub fn slice() -> AspectRatio {
    AspectRatio::new(PreserveAspectRatio::Slice, Alignment::Mid, Alignment::Mid)
  }
}

impl Default for AspectRatio {
  fn default() -> Self {
    AspectRatio::meet()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn intersect_clips_and_reports_no_overlap_as_empty() {
    let a = Rect::from_edges(-5, 10, 50, 40);
    let clipped = a.intersect(Size::new(30, 30));
    assert_eq!(clipped.edges::<i32>(), (0, 10, 30, 30));
    assert!(a.intersect(Rect::new((100, 100), (5, 5))).is_empty());
  }

  #[test]
  fn from_points_is_the_bounding_box() {
    let rect = Rect::from_points([(3.0, 9.0), (-1.0, 2.0), (7.0, 4.0)]);
    assert_eq!(rect.edges::<f32>(), (-1.0, 2.0, 7.0, 9.0));
    assert!(Rect::from_points(Vec::<PointF>::new()).is_empty());
  }

  #[test]
  fn map_point_meet_centers_the_content() {
    let source = Rect::new((0, 0), (100, 50));
    let mapped = source.map_point(PointF::new(100, 50), Size::new(200, 200), AspectRatio::meet());
    assert_eq!(mapped, PointF::new(200, 150));
    let stretched = source.map_point(PointF::new(100, 50), Size::new(200, 200), AspectRatio::none());
    assert_eq!(stretched, PointF::new(200, 200));
  }
}
