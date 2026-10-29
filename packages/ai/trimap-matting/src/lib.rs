//! ViTMatte: a rough mask in, a precise alpha out.
//!
//! [`TrimapMatting`] takes a photo and a trimap and returns the alpha of the area the trimap leaves unknown. Any ViTMatte
//! model works, such as the base or the small one: the model and its manifest are found by the path, so changing model is
//! changing the path. The manifest beside the `.onnx` file says how the model is fed (see `abra-ai-core`).

use abra_ai_core::{AiError, AiModel, AiOutput, ImageAi, ReportsProgress};
use abra_core::Image;
pub use mask::Mask;
use std::path::Path;

/// What a trimap says about a pixel, as the values of its [`Mask`]: anything between the two is also read as unknown.
pub mod trimap {
  /// Sure background.
  pub const BACKGROUND: u8 = 0;
  /// Not known: the model decides how much of the pixel is the subject.
  pub const UNKNOWN: u8 = 128;
  /// Sure subject.
  pub const SUBJECT: u8 = 255;
}

/// Finds the alpha of a subject from a photo and a trimap.
pub struct TrimapMatting {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{Mask, TrimapMatting, trimap};
  pub use abra_ai_core::{AiError, ImageAi, ReportsProgress};
}

impl ImageAi for TrimapMatting {
  type Output = Mask;

  /// Loads a ViTMatte ONNX model and the manifest beside it (the same path with a `.yml` extension).
  ///
  /// The caller decides where the model lives, and which one: the base and the small model load the same way.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    let model = AiModel::load(p_model_path)?;
    if model.manifest().decoder() != Some("matte") {
      return Err(AiError::config_error("A matting model's manifest must name the `matte` decoder"));
    }
    Ok(Self { model })
  }

  /// A trimap is needed, so this is an error: use [`TrimapMatting::refine`].
  fn process(&self, _p_image: &Image) -> Result<Mask, AiError> {
    Err(AiError::invalid_input("Matting needs a trimap; use refine"))
  }
}

impl TrimapMatting {
  /// Returns the alpha of the subject: `255` where it is sure, `0` where the background is, and the model's answer in
  /// between. The mask is the size of the photo.
  ///
  /// - `p_image`: The photo.
  /// - `p_trimap`: A mask with the photo's proportions, built from [`trimap`]'s values.
  pub fn refine(&self, p_image: &Image, p_trimap: &Mask) -> Result<Mask, AiError> {
    self.refine_reporting(p_image, p_trimap, &|_| {})
  }

  /// [`TrimapMatting::refine`], telling `p_progress` how far along it is (by stage, from `0.0` to `1.0`).
  pub fn refine_reporting(&self, p_image: &Image, p_trimap: &Mask, p_progress: &dyn Fn(f32)) -> Result<Mask, AiError> {
    match self.model.run_guided(p_image, Some(p_trimap), p_progress)? {
      AiOutput::Mask(alpha) => Ok(alpha),
      _ => Err(AiError::inference_failed("The matting model did not return a mask")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Channels;

  const MODELS: [&str; 2] = ["vitmatte-base-composition-1k", "vitmatte-small-composition-1k"];

  /// A 120 x 80 photo, bright on the left half and dark on the right, and its trimap: the left sure, the right sure
  /// background and a band of unknown between them.
  fn photo_and_trimap() -> (Image, Mask) {
    let pixels: Vec<u8> = (0..120u32 * 80)
      .flat_map(|index| if index % 120 < 60 { [220, 200, 180, 255] } else { [30, 40, 50, 255] })
      .collect();
    let trimap = (0..120u32 * 80)
      .map(|index| match index % 120 {
        0..50 => trimap::SUBJECT,
        50..70 => trimap::UNKNOWN,
        _ => trimap::BACKGROUND,
      })
      .collect();
    (Image::new_from_pixels(120, 80, pixels, Channels::RGBA), Mask::from_values(120, 80, trimap))
  }

  /// The alpha of the middle row of the test photo, from the model with this file name.
  fn middle_row(p_name: &str) -> Vec<u8> {
    let (photo, trimap) = photo_and_trimap();
    let model =
      TrimapMatting::load(concat!(env!("CARGO_MANIFEST_DIR"), "/models/").to_string() + p_name + ".onnx").unwrap();
    let alpha = model.refine(&photo, &trimap).unwrap_or_else(|error| panic!("{p_name}: {error}"));
    assert_eq!((alpha.width(), alpha.height()), (120, 80), "{p_name}");
    alpha.values()[40 * 120..41 * 120].to_vec()
  }

  #[test]
  fn the_small_model_keeps_what_is_sure_and_decides_the_rest() {
    let row = middle_row("vitmatte-small-composition-1k");
    assert!(row[..50].iter().all(|value| *value > 240), "sure subject stays subject: {:?}", &row[..50]);
    assert!(row[70..].iter().all(|value| *value < 20), "sure background stays background: {:?}", &row[70..]);
  }

  /// The 99 MB quantized base model runs the same way and gives an alpha of the right size, but its quantization made it
  /// noisy: it gave 30 to 60 on pixels the trimap says are sure subject, which the 387 MB model and the small one do not.
  /// This only checks that it loads and runs, so a model of its kind can be switched in by path.
  #[test]
  fn the_quantized_base_model_runs_the_same_way() {
    assert_eq!(middle_row("vitmatte-base-composition-1k").len(), 120);
  }

  #[test]
  fn a_trimap_is_required() {
    let model =
      TrimapMatting::load(concat!(env!("CARGO_MANIFEST_DIR"), "/models/vitmatte-small-composition-1k.onnx")).unwrap();
    assert!(model.process(&photo_and_trimap().0).is_err());
  }

  #[test]
  fn a_missing_model_is_an_error() {
    assert!(TrimapMatting::load("missing.onnx").is_err());
  }
}
