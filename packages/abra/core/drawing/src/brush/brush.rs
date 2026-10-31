use abra_core::{Area, Color, Fill, StrokeOpacity, StrokeWidth};

use crate::Blend;

/// A brush represents a drawing tool with a specific size.
/// It encapsulates properties such as size, shape (area), and fill color.
/// Brushes can be used for painting, drawing, and other graphical operations.
/// ```
/// use drawing::Brush;
/// use abra_core::{Color, Fill, Area};
/// let brush = Brush::new()
///   .with_size(10)
///   .with_area(Area::circle((0.0, 0.0), 5.0))
///   .with_color(Fill::solid(Color::red()));
/// ```
#[derive(Clone, Debug)]
pub struct Brush<'a> {
  /// The size of the brush. It is the width of a stroke, unless `width` says otherwise, and the size of a dab.
  size: u32,
  /// How wide a stroke along a path is, when it changes along the path. Nothing means the stroke is `size` wide all the way.
  width: Option<StrokeWidth>,
  /// The shape of the brush as an area.
  area: Area,
  /// The fill color of the brush.
  color: Fill<'a>,
  /// The hardness of the brush (0.0 to 1.0).
  hardness: f32,
  /// How opaque the brush is: one number, or stops along a stroke, where `0` is the start of the stroke and `1` is its end.
  opacity: StrokeOpacity,
  /// How the brush's color is combined with what is under it.
  blend: Blend,
}

impl<'a> Default for Brush<'a> {
  fn default() -> Self {
    Brush {
      size: 5,
      width: None,
      area: Area::circle((0, 0), 5),
      color: Fill::Solid(Color::black()),
      hardness: 0.0,
      opacity: StrokeOpacity::default(),
      blend: Blend::default(),
    }
  }
}

impl<'a> Brush<'a> {
  /// Creates a new Brush with default properties.
  /// By default, the size is `0`, the area is empty, and the color is black.
  /// # Defaults
  /// - size: `5`
  /// - area: circle with radius `5`
  /// - color: black
  /// - hardness: `0`
  /// - opacity: `1`
  pub fn new() -> Self {
    Self::default()
  }
  /// Sets the size of the brush: the size of a dab and the width of a stroke, the same all the way along it. It replaces a width set with
  /// [`Brush::with_width`].
  /// - `p_size`: The size to set for the brush.
  pub fn with_size(mut self, p_size: u32) -> Self {
    self.size = p_size;
    self.width = None;
    self
  }
  /// Sets how wide a stroke along a path is, where `0` is the start of the path and `1` is its end, so a stroke can taper or swell. A
  /// number is the same width all the way. A dab and a stroke that is not along a path (a stroke overlay, drawn through points) still use [`Brush::size`].
  /// - `p_width`: The width along the stroke.
  pub fn with_width(mut self, p_width: impl Into<StrokeWidth>) -> Self {
    self.width = Some(p_width.into());
    self
  }
  /// Sets the area of the brush.
  /// - `p_area`: The area to set for the brush.
  pub fn with_area(mut self, p_area: Area) -> Self {
    self.area = p_area;
    self
  }
  /// Sets the fill color of the brush.
  /// - `p_color`: The fill color to set for the brush. Can be a solid color, gradient, or image.
  pub fn with_color<T: Into<Fill<'a>>>(mut self, p_color: T) -> Self {
    self.color = p_color.into();
    self
  }
  /// Sets the hardness of the brush.
  /// - `p_hardness`: The hardness value to set for the brush (0.0 to 1.0). A value of 0.0 is soft causing a gradual fade, while 1.0 is hard with a sharp edge.
  pub fn with_hardness(mut self, p_hardness: f32) -> Self {
    self.hardness = p_hardness.clamp(0.0, 1.0);
    self
  }
  /// Sets the opacity of the brush.
  /// - `p_opacity`: A [`StrokeOpacity`]: how opaque a stroke is at each place along it, where `0` is its start and `1` is its end. A
  ///   number is the same opacity all the way, from `0.0` (fully transparent) to `1.0` (fully opaque, the brush's default). A dab, an area
  ///   fill and the stroke of a closed area have no line to run along, so they use the opacity at the start.
  pub fn with_opacity(mut self, p_opacity: impl Into<StrokeOpacity>) -> Self {
    self.opacity = p_opacity.into();
    self
  }
  /// Sets how the brush's color is combined with what is under it: any of abra's blend modes, or [`Blend::Additive`] to add its
  /// light. It defaults to `Normal`, plain painting.
  /// - `p_blend`: A [`Blend`], or a [`BlendMode`](abra_core::BlendMode).
  pub fn with_blend(mut self, p_blend: impl Into<Blend>) -> Self {
    self.blend = p_blend.into();
    self
  }
  /// Returns the size of the brush.
  pub fn size(&self) -> u32 {
    self.size
  }
  /// Returns how wide a stroke along a path is: what was set with [`Brush::with_width`], or the brush's size all the way along.
  pub fn width(&self) -> StrokeWidth {
    self.width.clone().unwrap_or_else(|| StrokeWidth::constant(self.size))
  }
  /// Returns the area shape of the brush.
  pub fn area(&self) -> &Area {
    &self.area
  }
  /// Returns the fill color of the brush.
  pub fn color(&self) -> &Fill<'_> {
    &self.color
  }
  /// Returns the hardness of the brush (0.0 to 1.0).
  pub fn hardness(&self) -> f32 {
    self.hardness
  }
  /// Returns how opaque the brush is along a stroke.
  pub fn opacity(&self) -> StrokeOpacity {
    self.opacity.clone()
  }
  /// Returns how the brush's color is combined with what is under it.
  pub fn blend(&self) -> Blend {
    self.blend
  }
}
