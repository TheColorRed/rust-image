use std::sync::Arc;

use crate::common::*;
use abra::canvas::prelude::Layer as AbraLayer;
use abra::{
  abra_core::BlendMode,
  canvas::prelude::Anchor,
  prelude::{Channels, Image, Transform},
  transform::prelude::{ResizeTarget, resize},
};

#[napi(object)]
/// Position object for setting a layer position.
pub struct Position {
  /// The x coordinate of the layer.
  pub x: i32,
  /// The y coordinate of the layer.
  pub y: i32,
}

#[napi]
pub struct Layer {
  pub(crate) inner: AbraLayer<'static>,
  pub(crate) project_id: String,
}

#[napi]
impl Layer {
  pub fn get_underlying_layer(&self) -> &AbraLayer<'static> {
    &self.inner
  }

  /// Internal-only mutable accessor used by bindings that need to mutate the
  /// underlying layer without going through the JS wrapper. This mirrors
  /// the immutable `get_underlying_layer` but returns a mutable reference.
  ///
  /// This accessor is intentionally `pub(crate)` — prefer `with_image_mut` for
  /// short-lived in-place image mutations to avoid lifetime/borrow issues.
  #[allow(dead_code)]
  pub(crate) fn get_underlying_layer_mut(&mut self) -> &mut AbraLayer<'static> {
    &mut self.inner
  }

  /// Run a closure with a mutable reference to the underlying Abra layer.
  ///
  /// This helps contain the mutable borrow for only the duration of the
  /// closure so callers can safely perform other mutable operations (like
  /// `mark_dirty`) afterwards without borrow conflicts.
  #[allow(dead_code)]
  pub(crate) fn with_underlying_layer_mut<R>(&mut self, f: impl FnOnce(&mut AbraLayer<'static>) -> R) -> R {
    let layer_ref = self.get_underlying_layer_mut();
    f(layer_ref)
  }

  #[napi]
  /// Gets the ID of the layer.
  /// @returns The ID of the layer.
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi(getter)]
  /// Gets the name of the layer.
  /// @returns The name of the layer.
  pub fn name(&self) -> String {
    self.inner.name().to_string()
  }

  #[napi]
  /// Gets the position of the layer.
  /// @returns The position of the layer.
  pub fn position(&self) -> Position {
    let (x, y) = self.inner.position();
    Position { x, y }
  }

  #[napi(getter)]
  /// Gets the opacity of the layer.
  /// @returns The opacity of the layer.
  pub fn opacity(&self) -> f64 {
    self.inner.opacity() as f64
  }

  #[napi(getter)]
  /// Gets the visibility of the layer.
  /// @returns The visibility of the layer.
  pub fn visible(&self) -> bool {
    self.inner.is_visible()
  }

  #[napi]
  /// Sets the name of the layer.
  /// @param name The new name of the layer.
  pub fn set_name(&mut self, name: String) -> &Self {
    self.inner.set_name(name);
    self
  }

  #[napi(
    ts_type = "(blendMode: 'normal' | 'darken' | 'average' | 'multiply' | 'color-burn' | 'linear-burn' | 'lighten' | 'screen' | 'color-dodge' | 'linear-dodge' | 'overlay' | 'soft-light' | 'hard-light' | 'vivid-light' | 'linear-light' | 'pin-light' | 'hard-mix' | 'difference' | 'exclusion' | 'subtract' | 'divide' | 'hue' | 'saturation' | 'color' | 'luminosity' | 'reflect' | 'glow' | 'phoenix' | 'negation' | 'grain-extract' | 'grain-merge'): this"
  )]
  /// Sets the blend mode of the layer.
  /// @param blendMode The new blend mode of the layer.
  pub fn set_blend_mode(&mut self, blend_mode: String) -> &Self {
    self.inner.set_blend_mode(BlendMode::from_name(&blend_mode).unwrap_or_default());
    self
  }

  #[napi]
  /// Sets the opacity of the layer.
  /// @param opacity The new opacity of the layer.
  pub fn set_opacity(&mut self, opacity: f64) -> &Self {
    self.inner.set_opacity(opacity as f32);
    self
  }

  #[napi]
  /// Sets the visibility of the layer.
  /// @param visible The new visibility of the layer.
  pub fn set_visibility(&mut self, visible: bool) -> &Self {
    self.inner.set_visible(visible);
    self
  }

  #[napi]
  /// Mark the parent canvas as needing recompose (forces a recompose on save).
  pub fn mark_dirty(&mut self) -> &Self {
    self.inner.mark_dirty();
    self
  }

  #[napi]
  /// Gets the underlying image of the layer.
  /// @param size Optional size to scale the image to. If only width or height is provided, the other dimension will be scaled proportionally.
  /// @returns The image of the layer.
  pub fn image_data(&self, size: Option<(u32, u32)>) -> ImageData {
    // Clone the image (cheap clone of Arc/Image) immediately so we don't hold
    // the ImageRef borrow for the duration of this method.
    let img = self.inner.with_image_mut(|i| i.clone());
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
      resize(ResizeTarget::Exact(abra::prelude::Size::new(new_width, new_height))).apply(&mut resized_img);
      resized_img
    };

    ImageData {
      data: Buffer::from(img.rgba().to_vec()),
      width: new_width,
      height: new_height,
    }
  }

  /// Clone and return the underlying image (owned). This avoids long-lived
  /// borrows when callers need to mutate the image in-place.
  pub fn clone_image(&self) -> Image {
    self.inner.with_image_mut(|img| img.clone())
  }

  #[napi]
  /// Sets a new image for the layer.
  /// @param data The new image data for the layer.
  pub fn set_image_data(&mut self, data: ImageData) -> &Self {
    let image_data = data.data;
    let image = Image::new_from_pixels(data.width, data.height, image_data.to_vec(), Channels::RGBA);
    let image = Arc::new(image);
    self.inner.set_image(image);
    self
  }

  #[napi]
  /// Sets the position of the layer.
  /// @param position The new position of the layer.
  pub fn set_position(&mut self, position: Position) -> &Self {
    // let (current_x, current_y) = self.layer.position();

    self.inner.set_anchor(None);
    self.inner.set_global_position(position.x, position.y);

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
    self.inner.set_anchor(anchor_enum);
    self
  }

  #[napi]
  /// Sets the rotation angle of the layer in degrees.
  /// @param angle The rotation angle in degrees.
  pub fn set_rotation(&mut self, angle: f64) -> &Self {
    if angle.is_nan() || angle == 0f64 {
      return self;
    }
    {
      // Rotate the actual image pixels in-place. This avoids borrowing the
      // underlying `LayerTransform` lifetime which can lead to complex
      // lifetime requirements across the binding boundary.
      self.inner.with_image_mut(|img| {
        img.rotate(angle, None);
      });
    }
    self
  }

  #[napi]
  /// Gets the metadata of the layer.
  /// @returns The layer metadata.
  pub fn metadata(&self) -> crate::metadata::LayerMetadata {
    layer_metadata(&self.inner, self.project_id.clone())
  }

  /// Internal helper: apply noise filter directly on the underlying image.
  pub fn apply_noise(&mut self, amount: f64, distribution: String, options: Option<&ApplyOptions>) {
    self.inner.with_image_mut(|img| {
      abra::filters::prelude::noise::noise(amount as f32)
        .with_distribution(match distribution.as_str() {
          "uniform" => abra::filters::prelude::noise::NoiseDistribution::Uniform,
          "gaussian" => abra::filters::prelude::noise::NoiseDistribution::Gaussian,
          _ => abra::filters::prelude::noise::NoiseDistribution::Uniform,
        })
        .with_options(options.unwrap_or(&ApplyOptions::default()).to_apply_options())
        .apply(img);
    });
    self.mark_dirty();
  }

  /// Internal helper: apply despeckle filter directly on the underlying image.
  pub fn apply_despeckle(&mut self, radius: f64, threshold: f64, options: Option<&ApplyOptions>) {
    let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
    self.inner.with_image_mut(|img| {
      abra::filters::prelude::noise::despeckle(radius as f32, threshold as f32).with_options(options).apply(img);
    });
    self.mark_dirty();
  }

  /// Internal helper: apply median filter directly on the underlying image.
  pub fn apply_median(&mut self, radius: f64, options: Option<&ApplyOptions>) {
    let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
    self.inner.with_image_mut(|img| {
      abra::filters::prelude::noise::median(radius as f32).with_options(options).apply(img);
    });
    self.mark_dirty();
  }
}

impl<'a> From<AbraLayer<'a>> for Layer {
  fn from(layer: AbraLayer<'a>) -> Self {
    // SAFETY: We erase the lifetime parameter here for the purposes of the
    // JS bindings. The underlying AbraLayer holds owned data (Images in
    // Arcs) so transmuting the lifetime is safe as long as the layers remain
    // properly owned by the Rust side.
    let inner: AbraLayer<'static> = unsafe { std::mem::transmute::<AbraLayer<'a>, AbraLayer<'static>>(layer) };
    Self {
      inner,
      project_id: String::new(),
    }
  }
}

pub fn layer_metadata(layer: &AbraLayer, project_id: String) -> crate::metadata::LayerMetadata {
  let (width, height) = layer.dimensions();
  let (x_offset, y_offset) = layer.position();
  crate::metadata::LayerMetadata {
    id: layer.id().to_string(),
    project_id,
    name: layer.name().to_string(),
    blend_mode: layer.blend_mode().name().to_string(),
    opacity: layer.opacity() as f64,
    visible: layer.is_visible(),
    order: layer.current_index().unwrap_or(0) as u32,
    adjustment_type: layer.adjustment_type().map(|t| t.to_string()),
    width,
    height,
    x: x_offset,
    y: y_offset,
    anchor: layer.anchor().map(|v| {
      match v {
        Anchor::TopLeft => "top-left",
        Anchor::TopCenter => "top-center",
        Anchor::TopRight => "top-right",
        Anchor::CenterLeft => "center-left",
        Anchor::Center => "center",
        Anchor::CenterRight => "center-right",
        Anchor::BottomLeft => "bottom-left",
        Anchor::BottomCenter => "bottom-center",
        Anchor::BottomRight => "bottom-right",
      }
      .to_string()
    }),
    angle: 0f64,
  }
}
