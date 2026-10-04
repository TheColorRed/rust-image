use abra::{
  abra_core::{Channels, Color, ColorStop, Gradient, Image},
  adjustments::prelude::color,
  adjustments::prelude::levels,
  filters::prelude::blur,
  options::prelude::Effect,
};
use napi::bindgen_prelude::{Buffer, Result};
use napi_derive::napi;

use crate::ImageData;

/// One color stop of a live gradient.
#[napi(object, js_name = "LiveGradientStop")]
pub struct LiveGradientStop {
  /// Position along the gradient, 0 to 1.
  pub position: f64,
  pub r: u32,
  pub g: u32,
  pub b: u32,
  pub a: u32,
}

/// An effect and its parameters for a live preview. `kind` is one of `brightness`, `contrast`, `gaussianBlur` or
/// `linearGradient`; only the fields that kind uses are read.
#[napi(object, js_name = "LiveEffect")]
pub struct LiveEffect {
  pub kind: String,
  /// brightness, contrast
  pub amount: Option<f64>,
  /// gaussianBlur, in pixels
  pub radius: Option<u32>,
  /// linearGradient, CSS-style degrees: 0 points up, 90 runs left to right
  pub angle: Option<f64>,
  /// linearGradient, 0 to 1
  pub opacity: Option<f64>,
  /// linearGradient
  pub stops: Option<Vec<LiveGradientStop>>,
}

fn apply_effect(effect: LiveEffect, image: &mut Image) -> Result<()> {
  match effect.kind.as_str() {
    "brightness" => levels::brightness(effect.amount.unwrap_or(0.0).round() as i32).apply(image),
    "contrast" => levels::contrast(effect.amount.unwrap_or(0.0)).apply(image),
    "gaussianBlur" => blur::gaussian_blur(effect.radius.unwrap_or(0)).apply(image),
    "linearGradient" => {
      let stops = effect
        .stops
        .unwrap_or_default()
        .into_iter()
        .map(|stop| {
          ColorStop::new(
            Color::from_rgba(
              stop.r.min(255) as u8,
              stop.g.min(255) as u8,
              stop.b.min(255) as u8,
              stop.a.min(255) as u8,
            ),
            stop.position as f32,
          )
        })
        .collect::<Vec<_>>();
      let gradient = if stops.is_empty() {
        Gradient::from_to(Color::transparent(), Color::transparent())
      } else {
        Gradient::new(stops)
      };
      color::LinearGradientEffect::angle(&gradient, effect.angle.unwrap_or(0.0) as f32)
        .with_opacity(effect.opacity.unwrap_or(1.0) as f32)
        .apply(image);
    }
    other => return Err(napi::Error::from_reason(format!("unknown live effect kind: {other}"))),
  }
  Ok(())
}

/// An image being edited interactively. Call `setEffects` on every parameter change and `poll` on each display frame;
/// only the latest state is ever returned. Run it in the main or a utility process and transfer frames to the
/// renderer as an `ArrayBuffer`.
#[napi(js_name = "LiveImage")]
pub struct LiveImage {
  width: u32,
  height: u32,
  original: Vec<u8>,
  pending: Option<ImageData>,
}

#[napi]
impl LiveImage {
  /// Starts a preview of `data` (`width * height` RGBA bytes). Pass an image already downscaled to screen size.
  #[napi(constructor)]
  pub fn new(width: u32, height: u32, data: Buffer) -> Result<Self> {
    if width == 0 || height == 0 || data.len() != width as usize * height as usize * 4 {
      return Err(napi::Error::from_reason(format!("expected {width}x{height} RGBA pixels, got {} bytes", data.len())));
    }
    Ok(Self {
      width,
      height,
      original: data.to_vec(),
      pending: None,
    })
  }

  /// Whether frames are rendered on the GPU (otherwise on the CPU, which blocks this call).
  #[napi]
  pub fn is_gpu(&self) -> bool {
    false
  }

  /// Replaces the effect chain and starts rendering it.
  #[napi]
  pub fn set_effects(&mut self, effects: Vec<LiveEffect>) -> Result<()> {
    let mut image = Image::new_from_pixels(self.width, self.height, &self.original, Channels::RGBA);
    for effect in effects {
      apply_effect(effect, &mut image)?;
    }
    self.pending = Some(ImageData::from_image(&image));
    Ok(())
  }

  /// The newest finished frame since the last call, or `null`. Never blocks.
  #[napi]
  pub fn poll(&mut self) -> Result<Option<ImageData>> {
    Ok(self.pending.take())
  }
}
