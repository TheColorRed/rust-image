//! The `tools` crate: the remover, which heals a blemish from the texture around it.
//!
//! A tool is made by its function and then applied to an image, as in the library:
//! `remover(RemoverShape.Circle.new(6)).withPosition(ShapePosition.Point.new({ x: 120, y: 80 })).apply(photo)`. The `with_*`
//! functions return a new remover, as the library's builders take and return the remover by value.
//!
//! Not exported: the alignment and atlas tools, and `Tool::apply_to`. The skin tools are in `beauty`. `with_position` takes the
//! [`ShapePosition`] that the library's `impl Into<ShapePosition>` turns a point or a pair of points into, and `with_path` takes
//! a list of points, one of the types the library's `impl IntoPathPoints` accepts.

use std::sync::Arc;

use abra::abra_core::{PointF, Units};
use abra::tools::prelude::{self as library, ShapePosition, Tool};

use crate::{Area, Image};

// How a length is measured: in pixels, in a physical unit at the image's resolution, or as a percentage of a reference length.
#[uniffi::remote(Enum)]
pub enum Units {
  Pixels(u32),
  Inches(u32),
  Centimeters(u32),
  Millimeters(u32),
  Points(u32),
  Picas(u32),
  Percent(u32),
}

// Where a shape is placed on the image: at a point, along a line from one point to another, or along a path of points.
#[uniffi::remote(Enum)]
pub enum ShapePosition {
  Point(PointF),
  Line(PointF, PointF),
  Path(Vec<PointF>),
}

/// The form of a [`Remover`]. A shape has a size but no location: where it lands is set on the [`Remover`]. The library's
/// shape holds an `Area` of its own, so this holds the handle of one.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum RemoverShape {
  Circle(f32),
  Rectangle(f32, f32),
  Area(Arc<Area>),
}

impl From<RemoverShape> for library::RemoverShape {
  fn from(shape: RemoverShape) -> Self {
    match shape {
      RemoverShape::Circle(radius) => library::RemoverShape::Circle(radius),
      RemoverShape::Rectangle(width, height) => library::RemoverShape::Rectangle(width, height),
      RemoverShape::Area(area) => library::RemoverShape::Area(area.core()),
    }
  }
}

/// Heals an image where a shape is placed on it, from the texture around the shape. Made by [`remover`].
#[derive(uniffi::Object)]
pub struct Remover(library::Remover);

/// A remover with this shape. It heals nothing until it has a position.
#[uniffi::export]
pub fn remover(shape: RemoverShape) -> Arc<Remover> {
  Arc::new(Remover(library::remover(shape.into())))
}

#[uniffi::export]
impl Remover {
  /// A copy of the remover with this hardness: how strongly it affects the image near the edge of the shape.
  pub fn with_hardness(&self, hardness: f64) -> Arc<Remover> {
    Arc::new(Remover(self.0.clone().with_hardness(hardness)))
  }

  /// A copy of the remover that samples a ring this far outside the shape. A circle with a radius of 20 pixels and a distance
  /// of 10 pixels samples out to a radius of 30.
  pub fn with_distance(&self, distance: Units) -> Arc<Remover> {
    Arc::new(Remover(self.0.clone().with_distance(distance)))
  }

  /// A copy of the remover that also heals at this position, after the ones it already has.
  pub fn with_position(&self, position: ShapePosition) -> Arc<Remover> {
    Arc::new(Remover(self.0.clone().with_position(position)))
  }

  /// A copy of the remover that also heals along a path through these points, as one stroke. One point is a dot and no points
  /// adds nothing.
  pub fn with_path(&self, points: Vec<PointF>) -> Arc<Remover> {
    Arc::new(Remover(self.0.clone().with_path(points)))
  }

  /// Heals the image in place at each position, in order.
  pub fn apply(&self, image: Arc<Image>) {
    image.edit(|image| Tool::apply(&self.0, image));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::{Channels, Image as CoreImage};

  /// A smooth gradient with a dark blemish in the middle.
  fn blemished() -> CoreImage {
    let (width, height) = (48u32, 48u32);
    let mut pixels = Vec::new();
    for y in 0..height {
      for x in 0..width {
        let dark = (x as f32 - 24.0).hypot(y as f32 - 24.0) < 3.0;
        let base = (80 + x * 2) as u8;
        pixels.extend_from_slice(&if dark { [10, 10, 10, 255] } else { [base, base, base, 255] });
      }
    }
    CoreImage::new_from_pixels(width, height, pixels, Channels::RGBA)
  }

  #[test]
  fn remover_matches_the_library_and_heals_the_blemish() {
    let handle = Arc::new(Image::from_core(blemished()));
    remover(RemoverShape::Circle(4.0))
      .with_distance(Units::Pixels(3))
      .with_position(ShapePosition::Point(PointF { x: 24.0, y: 24.0 }))
      .apply(handle.clone());

    let mut expected = blemished();
    library::remover(library::RemoverShape::Circle(4.0))
      .with_distance(Units::Pixels(3))
      .with_position(PointF { x: 24.0, y: 24.0 })
      .apply(&mut expected);

    assert_eq!(handle.core().rgba(), expected.rgba());
    assert_ne!(handle.core().rgba(), blemished().rgba(), "the blemish was healed");
  }

  #[test]
  fn a_remover_with_no_position_changes_nothing() {
    let handle = Arc::new(Image::from_core(blemished()));
    remover(RemoverShape::Rectangle(6.0, 6.0)).with_path(Vec::new()).apply(handle.clone());
    assert_eq!(handle.core().rgba(), blemished().rgba());
  }
}
