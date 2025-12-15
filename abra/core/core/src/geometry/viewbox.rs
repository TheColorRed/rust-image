use super::pointf::PointF;

#[derive(Debug, Clone, Copy, PartialEq)]
/// A ViewBox defines the coordinate system for a path, similar to SVG's viewBox.
/// Paths are defined in this abstract space and can be rendered at any size.
pub struct ViewBox {
  /// The x-coordinate of the top-left corner of the viewBox.
  pub x: f32,
  /// The y-coordinate of the top-left corner of the viewBox.
  pub y: f32,
  /// The width of the viewBox coordinate space.
  pub width: f32,
  /// The height of the viewBox coordinate space.
  pub height: f32,
}

impl ViewBox {
  /// Creates a new ViewBox with the given dimensions.
  pub fn new(p_x: f32, p_y: f32, p_width: f32, p_height: f32) -> ViewBox {
    ViewBox {
      x: p_x,
      y: p_y,
      width: p_width,
      height: p_height,
    }
  }

  /// Creates a ViewBox starting at the origin (0, 0).
  pub fn from_dimensions(p_width: f32, p_height: f32) -> ViewBox {
    ViewBox {
      x: 0.0,
      y: 0.0,
      width: p_width,
      height: p_height,
    }
  }

  /// Creates a square ViewBox starting at the origin.
  pub fn square(p_size: f32) -> ViewBox {
    ViewBox {
      x: 0.0,
      y: 0.0,
      width: p_size,
      height: p_size,
    }
  }

  /// Creates a standard unit ViewBox (0, 0, 1, 1) for normalized coordinates.
  pub fn unit() -> ViewBox {
    ViewBox {
      x: 0.0,
      y: 0.0,
      width: 1.0,
      height: 1.0,
    }
  }

  /// Calculates the scale factor to fit this viewBox into a viewport.
  pub fn scale_to_fit(&self, p_viewport_width: f32, p_viewport_height: f32) -> (f32, f32) {
    let scale_x = p_viewport_width / self.width;
    let scale_y = p_viewport_height / self.height;
    (scale_x, scale_y)
  }

  /// Calculates uniform scale to fit this viewBox into a viewport (maintains aspect ratio).
  pub fn uniform_scale_to_fit(&self, p_viewport_width: f32, p_viewport_height: f32) -> f32 {
    let (scale_x, scale_y) = self.scale_to_fit(p_viewport_width, p_viewport_height);
    scale_x.min(scale_y)
  }

  /// Transforms a point from viewBox coordinates to viewport coordinates.
  ///
  /// # Arguments
  /// * `p_point` - Point in viewBox coordinates
  /// * `p_viewport_width` - Target viewport width
  /// * `p_viewport_height` - Target viewport height
  /// * `p_aspect_ratio` - How to handle aspect ratio preservation
  pub fn map_point(
    &self, p_point: PointF, p_viewport_width: f32, p_viewport_height: f32, p_aspect_ratio: AspectRatio,
  ) -> PointF {
    match p_aspect_ratio.mode {
      PreserveAspectRatio::None => {
        // Stretch to fill - use non-uniform scaling
        let (scale_x, scale_y) = self.scale_to_fit(p_viewport_width, p_viewport_height);
        PointF::new((p_point.x - self.x) * scale_x, (p_point.y - self.y) * scale_y)
      }
      PreserveAspectRatio::Meet | PreserveAspectRatio::Slice => {
        // Uniform scaling
        let scale = if p_aspect_ratio.mode == PreserveAspectRatio::Meet {
          self.uniform_scale_to_fit(p_viewport_width, p_viewport_height)
        } else {
          // Slice: scale to cover (use max instead of min)
          let (scale_x, scale_y) = self.scale_to_fit(p_viewport_width, p_viewport_height);
          scale_x.max(scale_y)
        };

        let scaled_width = self.width * scale;
        let scaled_height = self.height * scale;

        // Calculate offset based on alignment
        let offset_x = match p_aspect_ratio.align_x {
          Alignment::Min => 0.0,
          Alignment::Mid => (p_viewport_width - scaled_width) * 0.5,
          Alignment::Max => p_viewport_width - scaled_width,
        };

        let offset_y = match p_aspect_ratio.align_y {
          Alignment::Min => 0.0,
          Alignment::Mid => (p_viewport_height - scaled_height) * 0.5,
          Alignment::Max => p_viewport_height - scaled_height,
        };

        PointF::new((p_point.x - self.x) * scale + offset_x, (p_point.y - self.y) * scale + offset_y)
      }
    }
  }

  /// Legacy method - transforms a point without aspect ratio control (non-uniform scaling).
  #[deprecated(note = "Use map_point with AspectRatio::none() instead")]
  pub fn map_point_simple(&self, p_point: PointF, p_viewport_width: f32, p_viewport_height: f32) -> PointF {
    let (scale_x, scale_y) = self.scale_to_fit(p_viewport_width, p_viewport_height);
    PointF::new((p_point.x - self.x) * scale_x, (p_point.y - self.y) * scale_y)
  }

  /// Returns the aspect ratio of the viewBox (width / height).
  pub fn aspect_ratio(&self) -> f32 {
    if self.height != 0.0 {
      self.width / self.height
    } else {
      1.0
    }
  }
}

impl Default for ViewBox {
  fn default() -> Self {
    ViewBox::unit()
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Defines how content should be scaled when the viewBox and viewport have different aspect ratios.
pub enum PreserveAspectRatio {
  /// Do not preserve aspect ratio, stretch to fill viewport.
  None,
  /// Preserve aspect ratio, scale to fit within viewport (may leave empty space).
  Meet,
  /// Preserve aspect ratio, scale to cover entire viewport (may crop content).
  Slice,
}

impl Default for PreserveAspectRatio {
  fn default() -> Self {
    PreserveAspectRatio::Meet
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Alignment options when preserving aspect ratio.
pub enum Alignment {
  /// Align to minimum (left/top).
  Min,
  /// Align to center.
  Mid,
  /// Align to maximum (right/bottom).
  Max,
}

impl Default for Alignment {
  fn default() -> Self {
    Alignment::Mid
  }
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
    AspectRatio {
      mode: PreserveAspectRatio::None,
      align_x: Alignment::Mid,
      align_y: Alignment::Mid,
    }
  }

  /// Preserve aspect ratio, fit within viewport, centered.
  pub fn meet() -> AspectRatio {
    AspectRatio {
      mode: PreserveAspectRatio::Meet,
      align_x: Alignment::Mid,
      align_y: Alignment::Mid,
    }
  }

  /// Preserve aspect ratio, cover entire viewport, centered.
  pub fn slice() -> AspectRatio {
    AspectRatio {
      mode: PreserveAspectRatio::Slice,
      align_x: Alignment::Mid,
      align_y: Alignment::Mid,
    }
  }
}

impl Default for AspectRatio {
  fn default() -> Self {
    AspectRatio::meet()
  }
}
