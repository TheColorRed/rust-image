use abra_core::{Area, Color, Fill};

/// A brush represents a drawing tool with a specific size.
/// It encapsulates properties such as size, shape (area), and fill color.
/// Brushes can be used for painting, drawing, and other graphical operations.
/// ```
/// use drawing::Brush;
/// use abra_core::{Color, Fill, Area};
/// let brush = Brush::new()
///   .with_size(10)
///   .with_area(Area::circle((0.0, 0.0), 5.0))
///   .with_color(Fill::Solid(Color::red()));
/// ```
pub struct Brush {
  /// The size of the brush.
  size: u32,
  /// The shape of the brush as an area.
  area: Area,
  /// The fill color of the brush.
  color: Fill,
  /// The hardness of the brush (0.0 to 1.0).
  hardness: f32,
  /// The opacity of the brush (0.0 to 1.0).
  opacity: f32,
}

impl Brush {
  /// Creates a new Brush with default properties.
  /// By default, the size is `0`, the area is empty, and the color is black.
  pub fn new() -> Self {
    Brush {
      size: 5,
      area: Area::circle((0, 0), 5),
      color: Fill::Solid(Color::black()),
      hardness: 0.0,
      opacity: 1.0,
    }
  }
  /// Sets the size of the brush.
  /// - `p_size`: The size to set for the brush.
  pub fn with_size(mut self, p_size: u32) -> Self {
    self.size = p_size;
    self
  }
  /// Sets the area of the brush.
  /// - `p_area`: The area to set for the brush.
  pub fn with_area(mut self, p_area: Area) -> Self {
    self.area = p_area;
    self
  }
  /// Sets the fill color of the brush.
  /// - `p_color`: The fill color to set for the brush.
  pub fn with_color(mut self, p_color: impl Into<Fill>) -> Self {
    self.color = p_color.into();
    self
  }
  /// Sets the hardness of the brush.
  /// - `p_hardness`: The hardness value to set for the brush (0.0 to 1.0).
  pub fn with_hardness(mut self, p_hardness: f32) -> Self {
    self.hardness = p_hardness.clamp(0.0, 1.0);
    self
  }
  /// Sets the opacity of the brush.
  /// - `p_opacity`: The opacity value to set for the brush (0.0 to 1.0).
  pub fn with_opacity(mut self, p_opacity: f32) -> Self {
    self.opacity = p_opacity.clamp(0.0, 1.0);
    self
  }
  /// Returns the size of the brush.
  pub fn size(&self) -> u32 {
    self.size
  }
  /// Returns the area shape of the brush.
  pub fn area(&self) -> &Area {
    &self.area
  }
  /// Returns the fill color of the brush.
  pub fn color(&self) -> &Fill {
    &self.color
  }
  /// Returns the hardness of the brush (0.0 to 1.0).
  pub fn hardness(&self) -> f32 {
    self.hardness
  }
  /// Returns the opacity of the brush (0.0 to 1.0).
  pub fn opacity(&self) -> f32 {
    self.opacity
  }
}
