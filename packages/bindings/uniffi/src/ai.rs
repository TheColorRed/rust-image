//! `AiModel`: any AI model, described by its manifest, from `abra-ai-core`.
//!
//! A model is two files, the `.onnx` and a `.yml` beside it that says how to feed the model and what its output means
//! (see `abra_ai_core::Manifest`). `AiModel::load(path)` reads both and `run(image)` does all the work in the library:
//! it prepares the picture, runs the model and reads the result. Nothing here knows a particular model, so a new model
//! of a kind the library can read is a new pair of files, not a new binding.
//!
//! What a run returns depends on the model, so it comes back as an [`AiOutput`] that says which [`AiOutputKind`] it is;
//! asking for another kind fails with `AiError`.
//!
//! The library's `Mask` is handed out as the bindings' `Mask` handle, so `Instance` and `LabelMap`, which hold masks,
//! are handles too. The library's `ImageModel` (image to image, such as super resolution) is here too, with its `on`
//! listener: `model.on({ update: (progress) => .. }).process(image)`. The pieces under `AiModel` and `ImageModel`
//! (`OnnxSession`, tiling, tensors) are not here.

use std::sync::Arc;

use abra::abra_core::PointF;
use abra_ai_core::{
  AiError, AiModel as Model, AiOutput as Output, Bounds, ControlParams, ImageAi, ImageAiWithControl,
  ImageModel as ImgModel, Instance as FoundInstance, LabelMap as Labels, Landmark, Landmarks, ReportsProgress,
};

use crate::{Image, Mask};

// Why an AI model could not load or run. Each case carries a message.
#[uniffi::remote(Error)]
#[uniffi(flat_error)]
pub enum AiError {
  ModelLoadFailed(String),
  InferenceFailed(String),
  InvalidInput(String),
  ModelNotFound(String),
  ConfigError(String),
  Other(String),
}

// A box in the pixels of the picture.
#[uniffi::remote(Record)]
pub struct Bounds {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

// One point of a face or body, with a relative depth. Smaller depths are closer to the camera.
#[uniffi::remote(Record)]
pub struct Landmark {
  pub point: PointF,
  pub depth: f32,
}

// Points found in a picture and how sure the model is that the thing is there, from `0` to `1`.
#[uniffi::remote(Record)]
pub struct Landmarks {
  pub landmarks: Vec<Landmark>,
  pub presence: f32,
}

/// What kind of result a model returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum AiOutputKind {
  /// One mask: how likely each pixel is to be what the model looks for.
  Mask,
  /// A box and mask for each thing found.
  Instances,
  /// The class of every pixel.
  Labels,
  /// Points, such as the mesh of a face.
  Landmarks,
}

/// One thing the model found: what it is, where, how sure the model is, and its soft silhouette.
#[derive(uniffi::Object)]
pub struct Instance(FoundInstance);

#[uniffi::export]
impl Instance {
  /// The class number the model gave it.
  pub fn class(&self) -> i32 {
    self.0.class
  }

  /// The box around it, in pixels of the picture.
  pub fn bounds(&self) -> Bounds {
    self.0.bounds
  }

  /// How sure the model is, from `0` to `1`.
  pub fn confidence(&self) -> f32 {
    self.0.confidence
  }

  /// The soft silhouette, the size of the picture.
  pub fn mask(&self) -> Arc<Mask> {
    Arc::new(Mask::from_core(self.0.mask.clone()))
  }
}

/// The class of every pixel of a picture.
#[derive(uniffi::Object)]
pub struct LabelMap(Labels);

#[uniffi::export]
impl LabelMap {
  /// The width in pixels.
  pub fn width(&self) -> u32 {
    self.0.width
  }

  /// The height in pixels.
  pub fn height(&self) -> u32 {
    self.0.height
  }

  /// The names of the classes, from the model's manifest. Empty when it lists none.
  pub fn classes(&self) -> Vec<String> {
    self.0.classes.clone()
  }

  /// The class number of every pixel, row by row from the top left.
  pub fn labels(&self) -> Vec<u8> {
    self.0.labels.clone()
  }

  /// The class number at a pixel, or nothing outside the picture.
  pub fn label_at(&self, x: u32, y: u32) -> Option<u8> {
    self.0.label_at(x, y)
  }

  /// The smallest box containing any of these class numbers, or nothing when none occur.
  pub fn bounds_for_classes(&self, classes: Vec<u8>) -> Option<Bounds> {
    let mut left = self.0.width;
    let mut top = self.0.height;
    let mut right = None;
    let mut bottom = None;
    for (index, label) in self.0.labels.iter().enumerate() {
      if !classes.contains(label) {
        continue;
      }
      let (x, y) = ((index as u32) % self.0.width, (index as u32) / self.0.width);
      left = left.min(x);
      top = top.min(y);
      right = Some(right.map_or(x, |edge: u32| edge.max(x)));
      bottom = Some(bottom.map_or(y, |edge: u32| edge.max(y)));
    }
    Some(Bounds {
      x: left,
      y: top,
      width: right? - left + 1,
      height: bottom? - top + 1,
    })
  }

  /// A mask that is `255` on the pixels of one class and `0` elsewhere.
  pub fn mask(&self, p_class: u8) -> Arc<Mask> {
    Arc::new(Mask::from_core(self.0.mask(p_class)))
  }

  /// A mask that is `255` on the pixels of any of the classes and `0` elsewhere.
  pub fn mask_of(&self, p_classes: Vec<u8>) -> Arc<Mask> {
    Arc::new(Mask::from_core(self.0.mask_of(&p_classes)))
  }

  /// A mask that is `255` on the pixels of any of the named classes and `0` elsewhere. A name that is not one of the
  /// model's classes selects nothing.
  pub fn mask_named(&self, p_names: Vec<String>) -> Arc<Mask> {
    let names: Vec<&str> = p_names.iter().map(String::as_str).collect();
    Arc::new(Mask::from_core(self.0.mask_named(&names)))
  }
}

/// What a model found in a picture. `kind()` says which of `mask`, `instances`, `labels` and `landmarks` to ask for.
#[derive(uniffi::Object)]
pub struct AiOutput(Output);

impl AiOutput {
  /// A handle for a result of the library.
  pub fn from_core(output: Output) -> Self {
    Self(output)
  }

  fn wrong_kind(&self, wanted: &str) -> AiError {
    AiError::invalid_input(format!("The model returned {:?}, not {wanted}", self.kind()))
  }
}

#[uniffi::export]
impl AiOutput {
  /// Which kind of result this is.
  pub fn kind(&self) -> AiOutputKind {
    match self.0 {
      Output::Mask(_) => AiOutputKind::Mask,
      Output::Instances(_) => AiOutputKind::Instances,
      Output::Labels(_) => AiOutputKind::Labels,
      Output::Landmarks(_) => AiOutputKind::Landmarks,
    }
  }

  /// The mask, when the kind is `Mask`.
  pub fn mask(&self) -> Result<Arc<Mask>, AiError> {
    match &self.0 {
      Output::Mask(mask) => Ok(Arc::new(Mask::from_core(mask.clone()))),
      _ => Err(self.wrong_kind("a mask")),
    }
  }

  /// The things found, when the kind is `Instances`.
  pub fn instances(&self) -> Result<Vec<Arc<Instance>>, AiError> {
    match &self.0 {
      Output::Instances(found) => Ok(found.iter().map(|instance| Arc::new(Instance(instance.clone()))).collect()),
      _ => Err(self.wrong_kind("instances")),
    }
  }

  /// The class of every pixel, when the kind is `Labels`.
  pub fn labels(&self) -> Result<Arc<LabelMap>, AiError> {
    match &self.0 {
      Output::Labels(labels) => Ok(Arc::new(LabelMap(labels.clone()))),
      _ => Err(self.wrong_kind("labels")),
    }
  }

  /// The points, when the kind is `Landmarks`.
  pub fn landmarks(&self) -> Result<Landmarks, AiError> {
    match &self.0 {
      Output::Landmarks(landmarks) => Ok(landmarks.clone()),
      _ => Err(self.wrong_kind("landmarks")),
    }
  }
}

/// An AI model, loaded from its `.onnx` file and the manifest that describes it.
#[derive(uniffi::Object)]
pub struct AiModel(Model);

impl AiModel {
  /// The model of the library that this holds.
  pub fn core(&self) -> &Model {
    &self.0
  }
}

#[uniffi::export]
impl AiModel {
  /// Loads a model and the `.yml` manifest beside it (the same path with a `.yml` extension).
  #[uniffi::constructor]
  pub fn load(onnx_path: String) -> Result<Self, AiError> {
    Ok(Self(Model::load(onnx_path)?))
  }

  /// Loads a model with a manifest that is not a file beside it, such as one a catalog lists. `manifest` is the YAML.
  #[uniffi::constructor]
  pub fn load_with_manifest(onnx_path: String, manifest: String) -> Result<Self, AiError> {
    Ok(Self(Model::load_with_manifest(onnx_path, &manifest)?))
  }

  /// The name of the model.
  pub fn name(&self) -> String {
    self.0.manifest().name().to_string()
  }

  /// What the model does.
  pub fn description(&self) -> String {
    self.0.manifest().description().to_string()
  }

  /// The decoder that reads the model's output, such as `semantic-mask` or `instance-segmentation`. It decides which
  /// `AiOutputKind` a run returns.
  pub fn decoder(&self) -> Option<String> {
    self.0.manifest().decoder().map(str::to_string)
  }

  /// The names of the classes the model tells apart. Empty when its manifest lists none.
  pub fn classes(&self) -> Vec<String> {
    self.0.manifest().classes().to_vec()
  }

  /// Runs the model on a picture of any size.
  pub fn run(&self, image: Arc<Image>) -> Result<Arc<AiOutput>, AiError> {
    Ok(Arc::new(AiOutput(self.0.run(&image.core())?)))
  }

  /// Listens to the progress of what the returned handle runs. The model runs in one piece, so progress is by stage.
  pub fn on(self: Arc<Self>, listener: Arc<dyn ProgressListener>) -> Arc<AiListening> {
    Arc::new(AiListening { model: self, listener })
  }
}

/// An AI model with a listener for the progress of what it runs, from `AiModel.on`.
#[derive(uniffi::Object)]
pub struct AiListening {
  model: Arc<AiModel>,
  listener: Arc<dyn ProgressListener>,
}

#[uniffi::export]
impl AiListening {
  /// Runs the model like `AiModel.run`, telling the listener how far along it is.
  pub fn process(&self, image: Arc<Image>) -> Result<Arc<AiOutput>, AiError> {
    let listener = &self.listener;
    let output = self.model.0.on(|progress| listener.update(progress)).process(&image.core())?;
    Ok(Arc::new(AiOutput(output)))
  }
}

/// Hears how far along a run is. uniffi has no bare closures, so the closure of the library's `ImageModel::on` is this
/// one-method interface here.
#[uniffi::export(with_foreign)]
pub trait ProgressListener: Send + Sync {
  /// Called with a value from `0` to `1`: `0` when the run starts, then the share finished, ending on `1`.
  fn update(&self, progress: f32);
}

/// An image-to-image model, such as super resolution, loaded from its `.onnx` file and the manifest beside it.
#[derive(uniffi::Object)]
pub struct ImageModel(ImgModel);

#[uniffi::export]
impl ImageModel {
  /// Loads a model and the `.yml` manifest beside it (the same path with a `.yml` extension).
  #[uniffi::constructor]
  pub fn load(onnx_path: String) -> Result<Self, AiError> {
    Ok(Self(ImgModel::load(onnx_path)?))
  }

  /// The name of the model.
  pub fn name(&self) -> String {
    self.0.name().to_string()
  }

  /// How many times larger the output is than the input.
  pub fn scale_factor(&self) -> f32 {
    self.0.scale_factor()
  }

  /// Whether the model takes control values.
  pub fn has_control(&self) -> bool {
    self.0.has_control()
  }

  /// Processes a picture, using the default control values when the model takes them.
  pub fn process(&self, image: Arc<Image>) -> Result<Arc<Image>, AiError> {
    Ok(Arc::new(Image::from_core(self.0.process(&image.core())?)))
  }

  /// Processes a picture with control values, one for each of the model's controls.
  pub fn process_with_control(&self, image: Arc<Image>, control: Vec<f32>) -> Result<Arc<Image>, AiError> {
    let output = self.0.process_with_control(&image.core(), &ControlParams::new(&control))?;
    Ok(Arc::new(Image::from_core(output)))
  }

  /// Listens to the progress of what the returned handle processes.
  pub fn on(self: Arc<Self>, listener: Arc<dyn ProgressListener>) -> Arc<Listening> {
    Arc::new(Listening { model: self, listener })
  }
}

/// A model with a listener for the progress of what it processes, from `ImageModel.on`.
#[derive(uniffi::Object)]
pub struct Listening {
  model: Arc<ImageModel>,
  listener: Arc<dyn ProgressListener>,
}

#[uniffi::export]
impl Listening {
  /// Processes a picture like `ImageModel.process`, telling the listener how far along it is.
  pub fn process(&self, image: Arc<Image>) -> Result<Arc<Image>, AiError> {
    let listener = &self.listener;
    let output = self.model.0.on(|progress| listener.update(progress)).process(&image.core())?;
    Ok(Arc::new(Image::from_core(output)))
  }

  /// Processes a picture like `ImageModel.process_with_control`, telling the listener how far along it is.
  pub fn process_with_control(&self, image: Arc<Image>, control: Vec<f32>) -> Result<Arc<Image>, AiError> {
    let listener = &self.listener;
    let output = self
      .model
      .0
      .on(|progress| listener.update(progress))
      .process_with_control(&image.core(), &ControlParams::new(&control))?;
    Ok(Arc::new(Image::from_core(output)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A model of the library by its folder and file name, or nothing when the file is only a Git LFS pointer.
  fn bundled(folder: &str, file: &str) -> Option<String> {
    let path = format!("{}/../../abra/ai/{folder}/models/{file}", env!("CARGO_MANIFEST_DIR"));
    (std::fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0) > 1_000_000).then_some(path)
  }

  fn photo(side: u32) -> Arc<Image> {
    Arc::new(Image::new(side, side))
  }

  #[test]
  fn a_missing_model_or_manifest_is_an_error_not_a_panic() {
    assert!(AiModel::load("missing.onnx".into()).is_err());
    assert!(AiModel::load_with_manifest("missing.onnx".into(), "decoder: nope\n".into()).is_err());
  }

  #[test]
  fn each_kind_of_model_returns_its_kind_through_the_bridge() {
    let cases = [
      ("body-segmentation", "selfie_multiclass_256x256.onnx", AiOutputKind::Mask),
      ("find-person", "yolo26n-seg.onnx", AiOutputKind::Instances),
      ("face-parsing", "resnet18.onnx", AiOutputKind::Labels),
      ("face-landmarks", "face_landmarks_detector.onnx", AiOutputKind::Landmarks),
    ];
    for (folder, file, kind) in cases {
      let Some(path) = bundled(folder, file) else { continue };
      let output = AiModel::load(path).unwrap().run(photo(64)).unwrap();
      assert_eq!(output.kind(), kind, "{folder}");
      match kind {
        AiOutputKind::Mask => assert_eq!(output.mask().unwrap().width(), 64),
        AiOutputKind::Instances => assert!(output.instances().is_ok()),
        AiOutputKind::Labels => assert_eq!(output.labels().unwrap().labels().len(), 64 * 64),
        AiOutputKind::Landmarks => assert_eq!(output.landmarks().unwrap().landmarks.len(), 478),
      }
      assert!(
        output.landmarks().is_err() || kind == AiOutputKind::Landmarks,
        "{folder}: asking for the wrong kind fails"
      );
    }
  }

  struct Recorder(std::sync::Mutex<Vec<f32>>);

  impl ProgressListener for Recorder {
    fn update(&self, progress: f32) {
      self.0.lock().unwrap().push(progress);
    }
  }

  #[test]
  fn a_listener_hears_progress_from_zero_to_one_through_the_bridge() {
    let Some(path) = bundled("super-resolution", "MewZoom-V1-2X.onnx") else { return };
    let recorder = Arc::new(Recorder(Default::default()));
    let output = Arc::new(ImageModel::load(path).unwrap()).on(recorder.clone()).process(photo(64)).unwrap();
    assert_eq!(output.core().dimensions::<u32>(), (128, 128));
    let heard = recorder.0.lock().unwrap();
    assert_eq!(heard.first(), Some(&0.0));
    assert_eq!(heard.last(), Some(&1.0));
    assert!(heard.windows(2).all(|pair| pair[0] <= pair[1]), "progress only goes forward: {heard:?}");
  }

  #[test]
  fn an_ai_model_reports_its_stages_through_the_bridge() {
    let Some(path) = bundled("body-segmentation", "selfie_multiclass_256x256.onnx") else { return };
    let recorder = Arc::new(Recorder(Default::default()));
    let model = Arc::new(AiModel::load(path).unwrap());
    let output = model.on(recorder.clone()).process(photo(64)).unwrap();
    assert_eq!(output.kind(), AiOutputKind::Mask);
    assert_eq!(*recorder.0.lock().unwrap(), vec![0.0, 0.1, 0.9, 1.0]);
  }

  #[test]
  fn a_model_reports_what_its_manifest_says() {
    let Some(path) = bundled("face-parsing", "resnet18.onnx") else { return };
    let model = AiModel::load(path).unwrap();
    assert_eq!(model.name(), "bisenet-resnet18-face-parsing");
    assert_eq!(model.classes().len(), 19);
    assert_eq!(model.decoder().as_deref(), Some("label-map"));
  }
}
