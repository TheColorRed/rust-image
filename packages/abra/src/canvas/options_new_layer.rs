//! Options for creating a new layer in a canvas.

use crate::{
  combine::blend::{self, RGBA},
  transform::ResizeAlgorithm,
};

use super::anchor::Anchor;

#[derive(Clone, Copy, Debug, PartialEq)]
/// How the image will be resized when added as a layer.
pub enum LayerSize {
  /// Maintains the original size of the image. Default behavior.
  Maintain,
  /// Scales the image as large as possible within its container without cropping or stretching the image.
  /// Defaults to the Auto resize algorithm.
  Contain(Option<ResizeAlgorithm>),
  /// Scales the image (while preserving its ratio) to the smallest possible size to fill the container (that is: both its height and width completely cover the container), leaving no empty space. If the proportions of the background differ from the element, the image is cropped either vertically or horizontally.
  /// Defaults to the Auto resize algorithm.
  Cover(Option<ResizeAlgorithm>),
  /// Resize the image to specific dimensions. If the size is larger than its container, it may be cropped.
  /// The width and height are specified in pixels.
  /// Defaults to the Auto resize algorithm.
  /// ```ignore
  /// let width = 800;
  /// let height = 600;
  /// Size::Specific(width, height, None);
  /// ```
  Specific(u32, u32, Option<ResizeAlgorithm>),
  /// Resize the image to a percentage of its original size.
  /// 0 to 1.0 represents 0% to 100%, values greater than 1.0 represent percentages over 100%.
  /// Defaults to the Auto resize algorithm.
  /// ```ignore
  /// let percentage = 0.5; // 50%
  /// Size::Percentage(percentage, None);
  /// ```
  Percentage(f32, Option<ResizeAlgorithm>),
}

/// Additional options for creating a new layer in a canvas.
#[derive(Clone)]
pub struct NewLayerOptions {
  /// Anchor point for the layer within the canvas.
  pub anchor: Option<Anchor>,
  /// The opacity of the layer.
  pub opacity: Option<f32>,
  /// The blend mode for the layer.
  pub blend_mode: Option<fn(RGBA, RGBA) -> RGBA>,
  /// How the image will be sized when added as a layer.
  /// The image can be left at its original size, stretched, or constrained to fit within the canvas.
  pub size: Option<LayerSize>,
}

impl Default for NewLayerOptions {
  fn default() -> Self {
    NewLayerOptions {
      anchor: Some(Anchor::Center),
      opacity: Some(1.0),
      blend_mode: Some(blend::normal),
      size: Some(LayerSize::Maintain),
    }
  }
}

impl NewLayerOptions {
  /// Creates a new `NewLayerOptions` with default values.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the size of the layer.
  /// The image can be left at its original size, stretched, or constrained to fit within the canvas.
  pub fn with_size(mut self, size: LayerSize) -> Self {
    self.size = Some(size);
    self
  }

  /// Sets the anchor point for the layer.
  /// The anchor point determines how the layer is positioned within the canvas when drawn.
  pub fn with_anchor(mut self, anchor: Anchor) -> Self {
    self.anchor = Some(anchor);
    self
  }

  /// Sets the opacity of the layer.
  /// The opacity value should be between 0.0 (completely transparent) and 1.0 (completely opaque).
  pub fn with_opacity(mut self, opacity: f32) -> Self {
    self.opacity = Some(opacity.clamp(0.0, 1.0));
    self
  }

  /// Sets the blend mode for the layer.
  /// The blend mode determines how the layer's pixels are combined with the pixels of the layers below it.
  /// This can be any of the predefined blend modes in the `blend` module.
  /// Or a custom blend function can be provided that takes two `RGBA` colors and returns a blended `RGBA` color.
  pub fn with_blend_mode(mut self, blend_mode: fn(RGBA, RGBA) -> RGBA) -> Self {
    self.blend_mode = Some(blend_mode);
    self
  }
}
