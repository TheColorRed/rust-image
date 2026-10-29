//! Robust Video Matting for a soft alpha of the person in a photo.
//!
//! [`Matting`] runs the RVM ONNX model on one picture and returns its alpha matte as an Abra [`Mask`]. The model has
//! recurrent states for video; a photo has no earlier frame, so each run starts from empty states and reads only the
//! alpha. The manifest is built in, so a caller only needs the `.onnx` file.

use abra_ai_core::{AiError, ImageAi, InputSpec, Manifest, ReportsProgress, onnx::OnnxSession};
use abra_core::{Image, TransformAlgorithm};
pub use mask::Mask;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/rvm_mobilenetv3_fp32.yml");

/// The start of a clip: each recurrent state is a single zero.
const EMPTY_STATE: [f32; 1] = [0.0];
const EMPTY_STATE_SHAPE: [usize; 4] = [1, 1, 1, 1];

/// Finds the soft alpha of the person in a picture.
pub struct Matting {
  session: OnnxSession,
  input: InputSpec,
  downsample_ratio: f32,
}

pub mod prelude {
  pub use crate::{Mask, Matting};
  pub use abra_ai_core::{AiError, ImageAi, ReportsProgress};
}

impl ImageAi for Matting {
  type Output = Mask;

  /// Loads a Robust Video Matting ONNX model from `p_model_path`.
  ///
  /// The caller decides where the model lives. It must have this model's input and output layout.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    let manifest = Manifest::parse(MANIFEST)?;
    Ok(Self {
      session: OnnxSession::from_file(p_model_path, None)?,
      input: InputSpec::from_manifest(&manifest)?,
      downsample_ratio: manifest.required_number("downsample_ratio")? as f32,
    })
  }

  /// Returns the alpha matte of `p_image`: `255` on the subject, `0` on the background and in between on hair and edges.
  /// The mask is the size of the picture.
  fn process(&self, p_image: &Image) -> Result<Mask, AiError> {
    self.process_reporting(p_image, &|_| {})
  }
}

impl ReportsProgress for Matting {
  fn process_reporting(&self, p_image: &Image, p_progress: &dyn Fn(f32)) -> Result<Mask, AiError> {
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return Err(AiError::invalid_input("Cannot run a model on an empty image"));
    }
    p_progress(0.0);
    let prepared = self.input.prepare(p_image);
    p_progress(0.1);
    let ratio = [self.downsample_ratio];
    let outputs = self.session.run_named(
      &[
        ("src", &prepared.shape, &prepared.data),
        ("r1i", &EMPTY_STATE_SHAPE, &EMPTY_STATE),
        ("r2i", &EMPTY_STATE_SHAPE, &EMPTY_STATE),
        ("r3i", &EMPTY_STATE_SHAPE, &EMPTY_STATE),
        ("r4i", &EMPTY_STATE_SHAPE, &EMPTY_STATE),
        ("downsample_ratio", &[1], &ratio),
      ],
      // `fgr`, then `pha`; the recurrent states after them are not needed.
      Some(2),
    )?;
    p_progress(0.9);
    let (shape, alpha) =
      outputs.get(1).ok_or_else(|| AiError::inference_failed("The matting model returned no alpha"))?;
    let (input_width, input_height) =
      (prepared.placement.input_width as usize, prepared.placement.input_height as usize);
    if shape != &[1, 1, input_height, input_width] || alpha.len() != input_width * input_height {
      return Err(AiError::inference_failed(format!("Unexpected alpha shape {shape:?}")));
    }
    let model_mask = Mask::from_values(
      prepared.placement.input_width,
      prepared.placement.input_height,
      alpha.iter().map(|value| (value * 255.0).round().clamp(0.0, 255.0) as u8).collect(),
    );
    let mask = prepared.placement.restore(&model_mask, width, height, TransformAlgorithm::Bilinear);
    p_progress(1.0);
    Ok(mask)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_built_in_manifest_describes_how_to_feed_the_model() {
    let manifest = Manifest::parse(MANIFEST).unwrap();
    assert!(InputSpec::from_manifest(&manifest).is_ok());
    assert!(manifest.required_number("downsample_ratio").is_ok());
  }

  #[test]
  fn a_missing_model_is_an_error() {
    assert!(Matting::load("missing.onnx").is_err());
  }

  #[test]
  fn the_model_gives_an_alpha_the_size_of_a_photo_that_is_not_square() {
    let model = Matting::load(concat!(env!("CARGO_MANIFEST_DIR"), "/models/rvm_mobilenetv3_fp32.onnx")).unwrap();
    let pixels: Vec<u8> =
      (0..300u32 * 200).flat_map(|index| [(index % 300) as u8, (index / 300) as u8, 128, 255]).collect();
    let photo = Image::new_from_pixels(300, 200, pixels, abra_core::Channels::RGBA);
    let alpha = model.process(&photo).unwrap();
    assert_eq!(alpha.dimensions::<u32>(), (300, 200));
  }
}
