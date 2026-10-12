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
  input::InputSpec,
  manifest::Manifest,
  onnx::OnnxSession,
};
use abra_core::Image;
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
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return Err(AiError::invalid_input("Cannot run a model on an empty image"));
    }
    let prepared = self.input.prepare(p_image);
    let outputs = self.session.run_multiple(&prepared.data, &prepared.shape)?;
    self.decoder.decode(&outputs, width, height, &prepared.placement)
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
