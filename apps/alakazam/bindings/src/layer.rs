use std::sync::Arc;

use crate::common::*;
use abra::{
  abra_core::blend::{self, blend_mode_name},
  canvas::prelude::Anchor,
  prelude::{Channels, Image, Resize, Rotate},
  transform::prelude::resize,
};

#[napi(object)]
/// Position object for setting a layer position.
pub struct Position {
  /// The x coordinate of the layer.
  pub x: Option<i32>,
  /// The y coordinate of the layer.
  pub y: Option<i32>,
}

#[napi]
pub struct Layer {
  pub(crate) layer: abra::canvas::prelude::Layer,
  pub(crate) project_id: String,
}

#[napi]
impl Layer {
  pub fn get_underlying_layer(&self) -> &abra::canvas::prelude::Layer {
    &self.layer
  }

  /// Internal-only mutable accessor used by bindings that need to mutate the
  /// underlying layer without going through the JS wrapper. This mirrors
  /// the immutable `get_underlying_layer` but returns a mutable reference.
  pub fn get_underlying_layer_mut(&mut self) -> &mut abra::canvas::prelude::Layer {
    &mut self.layer
  }

  #[napi(getter)]
  /// Gets the name of the layer.
  /// @returns The name of the layer.
  pub fn name(&self) -> String {
    self.layer.name().to_string()
  }

  #[napi(getter)]
  /// Gets the opacity of the layer.
  /// @returns The opacity of the layer.
  pub fn opacity(&self) -> f64 {
    self.layer.opacity() as f64
  }

  #[napi(getter)]
  /// Gets the visibility of the layer.
  /// @returns The visibility of the layer.
  pub fn visible(&self) -> bool {
    self.layer.is_visible()
  }

  #[napi]
  /// Sets the name of the layer.
  /// @param name The new name of the layer.
  pub fn set_name(&mut self, name: String) -> &Self {
    self.layer.set_name(name);
    self
  }

  #[napi(
    ts_type = "(blendMode: 'normal' | 'darken' | 'average' | 'multiply' | 'color-burn' | 'linear-burn' | 'lighten' | 'screen' | 'color-dodge' | 'linear-dodge' | 'overlay' | 'soft-light' | 'hard-light' | 'vivid-light' | 'linear-light' | 'pin-light' | 'hard-mix' | 'difference' | 'exclusion' | 'subtract' | 'divide' | 'hue' | 'saturation' | 'color' | 'luminosity' | 'reflect' | 'glow' | 'phoenix' | 'negation' | 'grain-extract' | 'grain-merge'): this"
  )]
  /// Sets the blend mode of the layer.
  /// @param blendMode The new blend mode of the layer.
  pub fn set_blend_mode(&mut self, blend_mode: String) -> &Self {
    let blend_fn = match blend_mode.as_str() {
      "normal" => blend::normal,
      "darken" => blend::darken,
      "reflect" => blend::reflect,
      "glow" => blend::glow,
      "phoenix" => blend::phoenix,
      "negation" => blend::negation,
      "grain-extract" => blend::grain_extract,
      "grain-merge" => blend::grain_merge,
      "darker-color" => blend::darker_color,
      "average" => blend::average,
      "multiply" => blend::multiply,
      "color-burn" => blend::color_burn,
      "linear-burn" => blend::linear_burn,
      "lighten" => blend::lighten,
      "lighter-color" => blend::lighter_color,
      "screen" => blend::screen,
      "color-dodge" => blend::color_dodge,
      "linear-dodge" => blend::linear_dodge,
      "overlay" => blend::overlay,
      "soft-light" => blend::soft_light,
      "hard-light" => blend::hard_light,
      "vivid-light" => blend::vivid_light,
      "linear-light" => blend::linear_light,
      "pin-light" => blend::pin_light,
      "hard-mix" => blend::hard_mix,
      "difference" => blend::difference,
      "exclusion" => blend::exclusion,
      "subtract" => blend::subtract,
      "divide" => blend::divide,
      "hue" => blend::hue,
      "saturation" => blend::saturation,
      "color" => blend::color,
      "luminosity" => blend::luminosity,
      _ => blend::normal,
    };
    self.layer.set_blend_mode(blend_fn);
    self.mark_dirty();
    self
  }

  #[napi]
  /// Sets the opacity of the layer.
  /// @param opacity The new opacity of the layer.
  pub fn set_opacity(&mut self, opacity: f64) -> &Self {
    self.layer.set_opacity(opacity as f32);
    self.mark_dirty();
    self
  }

  #[napi]
  /// Sets the visibility of the layer.
  /// @param visible The new visibility of the layer.
  pub fn set_visibility(&mut self, visible: bool) -> &Self {
    self.layer.set_visible(visible);
    self.mark_dirty();
    self
  }

  #[napi]
  /// Mark the parent canvas as needing recompose (forces a recompose on save).
  pub fn mark_dirty(&mut self) -> &Self {
    self.layer.mark_dirty();
    self
  }

  #[napi]
  /// Gets the underlying image of the layer.
  /// @param size Optional size to scale the image to. If only width or height is provided, the other dimension will be scaled proportionally.
  /// @returns The image of the layer.
  pub fn image_data(&self, size: Option<(u32, u32)>) -> ImageData {
    let img = self.layer.image();
    let (width, height) = img.dimensions();

    let (new_width, new_height) = if let Some((target_width, target_height)) = size {
      match (target_width, target_height) {
        (0, 0) => (width, height),
        (w, 0) => {
          let aspect_ratio = height as f32 / width as f32;
          (w, (w as f32 * aspect_ratio) as u32)
        }
        (0, h) => {
          let aspect_ratio = width as f32 / height as f32;
          ((h as f32 * aspect_ratio) as u32, h)
        }
        (w, h) => (w, h),
      }
    } else {
      (width, height)
    };

    let img = if new_width == width && new_height == height {
      img.clone()
    } else {
      let mut resized_img = img.clone();
      resize(&mut resized_img, new_width, new_height, None);
      resized_img
    };

    ImageData {
      data: Buffer::from(img.rgba().to_vec()),
      width: new_width,
      height: new_height,
    }
  }

  #[napi]
  /// Sets a new image for the layer.
  /// @param data The image data as a Buffer.
  /// @param width The width of the image.
  /// @param height The height of the image.
  pub fn set_image_data(&mut self, data: ImageData) -> &Self {
    let image_data = data.data;
    let image = Image::new_from_pixels(data.width, data.height, image_data.to_vec(), Channels::RGBA);
    let image = Arc::new(image);
    self.layer.set_image(image);
    self
  }

  #[napi]
  /// Sets the position of the layer.
  /// @param position The new position of the layer.
  pub fn set_position(&mut self, position: Position) -> &Self {
    let (current_x, current_y) = self.layer.position();

    self.layer.set_anchor(None);

    if let (Some(x), Some(y)) = (position.x, position.y) {
      self.layer.set_global_position(x, y);
    } else if let Some(x) = position.x {
      self.layer.set_global_position(x, current_y);
    } else if let Some(y) = position.y {
      self.layer.set_global_position(current_x, y);
    }

    self
  }

  #[napi(
    ts_type = "(anchor: 'top-left' | 'top-center' | 'top-right' | 'center-left' | 'center' | 'center-right' | 'bottom-left' | 'bottom-center' | 'bottom-right' | null): this"
  )]
  /// Sets the anchor point of the layer.
  /// @param anchor The new anchor point of the layer. Can be one of: '
  pub fn set_anchor(&mut self, anchor: Option<String>) -> &Self {
    let anchor_enum = match anchor.as_deref() {
      Some("top-left") => Some(Anchor::TopLeft),
      Some("top-center") => Some(Anchor::TopCenter),
      Some("top-right") => Some(Anchor::TopRight),
      Some("center-left") => Some(Anchor::CenterLeft),
      Some("center") => Some(Anchor::Center),
      Some("center-right") => Some(Anchor::CenterRight),
      Some("bottom-left") => Some(Anchor::BottomLeft),
      Some("bottom-center") => Some(Anchor::BottomCenter),
      Some("bottom-right") => Some(Anchor::BottomRight),
      _ => None,
    };
    self.layer.set_anchor(anchor_enum);
    self
  }

  #[napi]
  pub fn set_rotation(&mut self, angle: f64) -> &Self {
    if angle.is_nan() || angle == 0f64 {
      return self;
    }
    self.layer.transform().rotate(angle, None);
    self
  }

  #[napi]
  /// Gets the metadata of the layer.
  /// @returns The layer metadata.
  pub fn metadata(&self) -> crate::metadata::LayerMetadata {
    let (width, height) = self.layer.dimensions::<u32>();
    let (x_offset, y_offset) = self.layer.position();
    crate::metadata::LayerMetadata {
      id: self.layer.id().to_string(),
      project_id: self.project_id.clone(),
      name: self.layer.name().to_string(),
      blend_mode: blend_mode_name(self.layer.blend_mode()).0.to_string(),
      opacity: self.layer.opacity() as f64,
      visible: self.layer.is_visible(),
      order: self.layer.current_index().unwrap_or(0) as u32,
      adjustment_type: self.layer.adjustment_type().map(|t| t.to_string()),
      width,
      height,
      x: x_offset,
      y: y_offset,
      angle: 0f64,
    }
  }
}
