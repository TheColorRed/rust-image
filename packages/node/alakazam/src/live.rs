use abra::live::{EffectSpec, GradientStop, LiveImage as AbraLiveImage};
use napi::bindgen_prelude::*;
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

fn spec(p_effect: LiveEffect) -> Result<EffectSpec> {
  Ok(match p_effect.kind.as_str() {
    "brightness" => EffectSpec::Brightness {
      amount: p_effect.amount.unwrap_or(0.0).round() as i32,
    },
    "contrast" => EffectSpec::Contrast {
      amount: p_effect.amount.unwrap_or(0.0),
    },
    "gaussianBlur" => EffectSpec::GaussianBlur {
      radius: p_effect.radius.unwrap_or(0),
    },
    "linearGradient" => EffectSpec::LinearGradient {
      stops: p_effect
        .stops
        .unwrap_or_default()
        .into_iter()
        .map(|s| GradientStop {
          position: s.position as f32,
          r: s.r.min(255) as u8,
          g: s.g.min(255) as u8,
          b: s.b.min(255) as u8,
          a: s.a.min(255) as u8,
        })
        .collect(),
      angle: p_effect.angle.unwrap_or(0.0) as f32,
      opacity: p_effect.opacity.unwrap_or(1.0) as f32,
    },
    other => return Err(Error::from_reason(format!("unknown live effect kind: {other}"))),
  })
}

/// An image being edited interactively. Call `setEffects` on every parameter change and `poll` on each display frame;
/// only the latest state is ever returned. Run it in the main or a utility process and transfer frames to the
/// renderer as an `ArrayBuffer`.
#[napi(js_name = "LiveImage")]
pub struct LiveImage {
  inner: AbraLiveImage,
}

#[napi]
impl LiveImage {
  /// Starts a preview of `data` (`width * height` RGBA bytes). Pass an image already downscaled to screen size.
  #[napi(constructor)]
  pub fn new(width: u32, height: u32, data: Buffer) -> Result<Self> {
    let inner = AbraLiveImage::new(width, height, data.to_vec()).map_err(Error::from_reason)?;
    Ok(Self { inner })
  }

  /// Whether frames are rendered on the GPU (otherwise on the CPU, which blocks this call).
  #[napi]
  pub fn is_gpu(&self) -> bool {
    self.inner.is_gpu()
  }

  /// Replaces the effect chain and starts rendering it.
  #[napi]
  pub fn set_effects(&mut self, effects: Vec<LiveEffect>) -> Result<()> {
    let specs = effects.into_iter().map(spec).collect::<Result<Vec<_>>>()?;
    self.inner.set_effects(&specs).map_err(Error::from_reason)
  }

  /// The newest finished frame since the last call, or `null`. Never blocks.
  #[napi]
  pub fn poll(&mut self) -> Result<Option<ImageData>> {
    let frame = self.inner.poll().map_err(Error::from_reason)?;
    Ok(frame.map(|frame| ImageData {
      data: Buffer::from(frame.pixels),
      width: frame.width,
      height: frame.height,
    }))
  }
}
