//! Any model, described by its manifest.
//!
//! [`AiModel`] is the plugin side of the AI crates. It reads a model's manifest, feeds the picture to the ONNX file the
//! way the manifest says ([`InputSpec`]), runs it and reads the result with the decoder the manifest names
//! ([`Decoder`]). All the work is here; a caller only names a model and a picture.
//!
//! ```ignore
//! let model = AiModel::load("models/yolo26n-seg.onnx")?; // reads yolo26n-seg.yml beside it
//! match model.run(&photo)? {
//!   AiOutput::Instances(people) => println!("{} people", people.len()),
//!   _ => {}
//! }
//! ```

use crate::{
  decode::{AiOutput, Decoder},
  error::AiError,
  image_ai::ImageAi,
  input::InputSpec,
  manifest::Manifest,
  onnx::OnnxSession,
  progress::ReportsProgress,
};
use abra_core::Image;
use mask::Mask;
use std::path::Path;

/// An ONNX model together with how to feed it and how to read what it returns.
pub struct AiModel {
  session: OnnxSession,
  manifest: Manifest,
  input: InputSpec,
  decoder: Decoder,
}

impl AiModel {
  /// Loads a model and the manifest beside it (the `.onnx` path with a `.yml` extension).
  ///
  /// - `p_onnx_path`: Path to the ONNX model file.
  pub fn load(p_onnx_path: impl AsRef<Path>) -> Result<Self, AiError> {
    let manifest = Manifest::load(p_onnx_path.as_ref())?;
    Self::with_manifest(p_onnx_path, manifest)
  }

  /// Loads a model with a manifest that did not come from a file, such as one a catalog lists or an app ships.
  ///
  /// - `p_onnx_path`: Path to the ONNX model file.
  /// - `p_manifest`: The YAML of the model's manifest.
  pub fn load_with_manifest(p_onnx_path: impl AsRef<Path>, p_manifest: &str) -> Result<Self, AiError> {
    Self::with_manifest(p_onnx_path, Manifest::parse(p_manifest)?)
  }

  fn with_manifest(p_onnx_path: impl AsRef<Path>, p_manifest: Manifest) -> Result<Self, AiError> {
    let input = InputSpec::from_manifest(&p_manifest)?;
    let decoder = Decoder::from_manifest(&p_manifest)?;
    Ok(Self {
      session: OnnxSession::from_file(p_onnx_path, None)?,
      manifest: p_manifest,
      input,
      decoder,
    })
  }

  /// The manifest the model was loaded with.
  pub fn manifest(&self) -> &Manifest {
    &self.manifest
  }

  /// Runs the model on a picture.
  ///
  /// - `p_image`: The picture. Any size; it is fitted to the model's input.
  pub fn run(&self, p_image: &Image) -> Result<AiOutput, AiError> {
    self.run_reporting(p_image, &|_| {})
  }

  /// Runs the model on a picture and tells `p_progress` how far along it is.
  ///
  /// The model runs in one piece, so progress is by stage, not measured: `0.0` at the start, `0.1` once the picture is
  /// prepared, `0.9` once the model has run (nearly all the time goes there) and `1.0` once the result is read.
  pub fn run_reporting(&self, p_image: &Image, p_progress: &dyn Fn(f32)) -> Result<AiOutput, AiError> {
    self.run_guided(p_image, None, p_progress)
  }

  /// Runs a model that takes a guide mask with the picture, such as a matting model and its trimap (a manifest with
  /// `input_guide`). Progress is reported as in [`AiModel::run_reporting`].
  ///
  /// - `p_image`: The picture. Any size; it is fitted to the model's input.
  /// - `p_guide`: The guide, which has the picture's proportions; it is scaled to the picture and fitted with it.
  pub fn run_guided(
    &self, p_image: &Image, p_guide: Option<&Mask>, p_progress: &dyn Fn(f32),
  ) -> Result<AiOutput, AiError> {
    if self.input.guide != p_guide.is_some() {
      return Err(AiError::invalid_input(if self.input.guide {
        "This model needs a guide mask with the picture"
      } else {
        "This model takes no guide mask"
      }));
    }
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return Err(AiError::invalid_input("Cannot run a model on an empty image"));
    }
    p_progress(0.0);
    let prepared = self.input.prepare_guided(p_image, p_guide);
    p_progress(PREPARED);
    let outputs = self.session.run_multiple(&prepared.data, &prepared.shape)?;
    p_progress(RAN);
    let output = self.decoder.decode(&outputs, width, height, &prepared.placement)?;
    p_progress(1.0);
    Ok(output)
  }
}

/// How far along a run is once the picture is prepared. Running the model is nearly all of the time, so it gets nearly all
/// of the range.
const PREPARED: f32 = 0.1;
/// How far along a run is once the model has run.
const RAN: f32 = 0.9;

impl ImageAi for AiModel {
  type Output = AiOutput;

  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    AiModel::load(p_model_path)
  }

  fn process(&self, p_image: &Image) -> Result<AiOutput, AiError> {
    self.run(p_image)
  }
}

impl ReportsProgress for AiModel {
  fn process_reporting(&self, p_input: &Image, p_progress: &dyn Fn(f32)) -> Result<AiOutput, AiError> {
    self.run_reporting(p_input, p_progress)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_missing_model_or_manifest_is_an_error() {
    assert!(AiModel::load("missing.onnx").is_err());
    assert!(AiModel::load_with_manifest("missing.onnx", "name: x\n").is_err());
  }

  #[test]
  fn a_bad_manifest_fails_before_the_model_is_read() {
    let error = AiModel::load_with_manifest("missing.onnx", "decoder: nope\nparams:\n  input_size: 8\n").err().unwrap();
    assert!(error.to_string().contains("Unknown decoder"), "{error}");
  }
}
