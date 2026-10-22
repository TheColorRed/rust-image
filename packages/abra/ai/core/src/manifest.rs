//! The manifest that describes a model.
//!
//! Every model ships a `.yml` next to its `.onnx`. The manifest is what makes a model a plugin: it says what the model is
//! (`decoder`), how to feed it (the `input_*` params) and how to read what it returns (the decoder's params), so a new
//! model of a known kind needs no code, only its two files.
//!
//! ```yaml
//! name: selfie_multiclass_256x256
//! description: Six-class selfie segmentation
//! decoder: semantic-mask
//!
//! params:
//!   input_size: 256
//!   input_layout: nhwc
//!   class_count: 6
//!   mask_classes: [2, 3]
//!
//! classes: [background, hair, body skin, face skin, clothes, accessories]
//! ```

use crate::{
  error::AiError,
  image_model::{ControlParameter, ControlSpec},
};
use saphyr::{LoadableYamlNode, Yaml};
use std::collections::HashMap;
use std::path::Path;

/// One value in the `params` of a manifest.
#[derive(Clone, Debug, PartialEq)]
enum Param {
  Number(f64),
  Text(String),
  List(Vec<f64>),
}

/// One entry of a list (or one section) in a manifest, such as one of its `inputs`: names with a number, a text or a list of numbers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
  values: HashMap<String, Param>,
}

impl Entry {
  fn from_yaml(p_value: &Yaml) -> Option<Self> {
    let mapping = p_value.as_mapping()?;
    Some(Self {
      values: mapping.iter().filter_map(|(key, value)| Some((key.as_str()?.to_string(), param(value)?))).collect(),
    })
  }

  /// A number in the entry.
  pub fn number(&self, p_key: &str) -> Option<f64> {
    match self.values.get(p_key) {
      Some(Param::Number(value)) => Some(*value),
      _ => None,
    }
  }

  /// A text in the entry.
  pub fn text(&self, p_key: &str) -> Option<&str> {
    match self.values.get(p_key) {
      Some(Param::Text(value)) => Some(value),
      _ => None,
    }
  }

  /// A list of numbers in the entry.
  pub fn list(&self, p_key: &str) -> Option<&[f64]> {
    match self.values.get(p_key) {
      Some(Param::List(values)) => Some(values),
      _ => None,
    }
  }
}

/// What a model's `.yml` says about it.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
  name: String,
  description: String,
  decoder: Option<String>,
  classes: Vec<String>,
  params: HashMap<String, Param>,
  sections: HashMap<String, Vec<Entry>>,
  control: Option<ControlSpec>,
}

fn number(p_value: &Yaml) -> Option<f64> {
  p_value.as_floating_point().or_else(|| p_value.as_integer().map(|integer| integer as f64))
}

fn param(p_value: &Yaml) -> Option<Param> {
  if let Some(value) = number(p_value) {
    Some(Param::Number(value))
  } else if let Some(value) = p_value.as_str() {
    Some(Param::Text(value.to_string()))
  } else {
    p_value.as_vec().map(|items| Param::List(items.iter().filter_map(number).collect()))
  }
}

impl Manifest {
  /// Reads the `.yml` next to an ONNX file (same name, `.yml` extension).
  ///
  /// - `p_onnx_path`: Path to the ONNX model file.
  pub fn load(p_onnx_path: impl AsRef<Path>) -> Result<Self, AiError> {
    let yaml_path = p_onnx_path.as_ref().with_extension("yml");
    let text = std::fs::read_to_string(&yaml_path).map_err(|e| {
      AiError::model_load_failed(format!("Model manifest {} could not be read: {e}", yaml_path.display()))
    })?;
    Self::parse(&text)
  }

  /// Reads a manifest from its text.
  ///
  /// - `p_text`: The YAML of the manifest.
  pub fn parse(p_text: &str) -> Result<Self, AiError> {
    let docs =
      Yaml::load_from_str(p_text).map_err(|e| AiError::model_load_failed(format!("Invalid manifest YAML: {e}")))?;
    let doc = docs.first().ok_or_else(|| AiError::model_load_failed("The model manifest is empty"))?;
    Ok(Self::from_yaml(doc))
  }

  pub(crate) fn from_yaml(p_doc: &Yaml) -> Self {
    let text = |key: &str| p_doc.as_mapping_get(key).and_then(Yaml::as_str).map(str::to_string);
    let mut params = HashMap::new();
    // A manifest without a `params` block keeps its settings at the top, as the image models' manifests may.
    let mapping = p_doc.as_mapping_get("params").and_then(Yaml::as_mapping).or_else(|| p_doc.as_mapping());
    if let Some(mapping) = mapping {
      for (key, value) in mapping.iter() {
        let Some(key) = key.as_str() else { continue };
        let Some(param) = param(value) else { continue };
        params.insert(key.to_string(), param);
      }
    }
    // Lists of mappings (`inputs`) and mappings (`window`) at the top are sections, whose entries have their own settings.
    let mut sections = HashMap::new();
    for (key, value) in p_doc.as_mapping().into_iter().flat_map(|mapping| mapping.iter()) {
      let Some(key) = key.as_str().filter(|key| !matches!(*key, "params" | "control")) else { continue };
      let entries: Vec<Entry> = match value.as_vec() {
        Some(items) => items.iter().filter_map(Entry::from_yaml).collect(),
        None => Entry::from_yaml(value).into_iter().collect(),
      };
      if !entries.is_empty() {
        sections.insert(key.to_string(), entries);
      }
    }
    Self {
      name: text("name").unwrap_or_else(|| "Unknown Model".into()),
      description: text("description").unwrap_or_default(),
      decoder: text("decoder"),
      classes: p_doc
        .as_mapping_get("classes")
        .and_then(Yaml::as_vec)
        .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
        .unwrap_or_default(),
      params,
      sections,
      control: Self::control_from_yaml(p_doc),
    }
  }

  /// The `control` block of an image model: the extra numbers it takes beside the picture.
  fn control_from_yaml(p_doc: &Yaml) -> Option<ControlSpec> {
    let control = p_doc.as_mapping_get("control").filter(|control| control.as_mapping().is_some())?;
    let size = control.as_mapping_get("size").and_then(Yaml::as_integer).unwrap_or(0).max(0) as usize;
    if size == 0 {
      return None;
    }
    let defaults = control
      .as_mapping_get("defaults")
      .and_then(Yaml::as_vec)
      .map(|values| values.iter().filter_map(|value| number(value).map(|value| value as f32)).collect())
      .unwrap_or_else(|| vec![0.5; size]);
    let parameters = control
      .as_mapping_get("parameters")
      .and_then(Yaml::as_vec)
      .map(|items| {
        items
          .iter()
          .filter_map(|item| {
            Some(ControlParameter {
              name: item.as_mapping_get("name")?.as_str()?.to_string(),
              description: item.as_mapping_get("description").and_then(Yaml::as_str).unwrap_or("").to_string(),
              index: item.as_mapping_get("index")?.as_integer()? as usize,
            })
          })
          .collect()
      })
      .unwrap_or_default();
    Some(ControlSpec {
      size,
      defaults,
      parameters,
    })
  }

  /// The name of the model.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// What the model does.
  pub fn description(&self) -> &str {
    &self.description
  }

  /// Which decoder reads the model's output, such as `semantic-mask` or `instance-segmentation`.
  pub fn decoder(&self) -> Option<&str> {
    self.decoder.as_deref()
  }

  /// The names of the classes the model tells apart, in the order of its output channels.
  pub fn classes(&self) -> &[String] {
    &self.classes
  }

  /// The extra numbers an image model takes beside the picture, from the `control` block. `None` when it has none.
  pub fn control(&self) -> Option<&ControlSpec> {
    self.control.as_ref()
  }

  /// The entries of a list in the manifest, such as its `inputs`, or the one entry of a mapping such as its `window`. Empty when the
  /// manifest has none.
  pub fn section(&self, p_key: &str) -> &[Entry] {
    self.sections.get(p_key).map(Vec::as_slice).unwrap_or_default()
  }

  /// A number in `params`.
  pub fn number(&self, p_key: &str) -> Option<f64> {
    match self.params.get(p_key) {
      Some(Param::Number(value)) => Some(*value),
      _ => None,
    }
  }

  /// A text in `params`.
  pub fn text(&self, p_key: &str) -> Option<&str> {
    match self.params.get(p_key) {
      Some(Param::Text(value)) => Some(value),
      _ => None,
    }
  }

  /// A list of numbers in `params`.
  pub fn list(&self, p_key: &str) -> Option<&[f64]> {
    match self.params.get(p_key) {
      Some(Param::List(values)) => Some(values),
      _ => None,
    }
  }

  /// A number in `params` that the decoder cannot work without.
  pub fn required_number(&self, p_key: &str) -> Result<f64, AiError> {
    self.number(p_key).ok_or_else(|| {
      AiError::config_error(format!("The manifest of {} has no number `{p_key}` in its params", self.name))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const TEXT: &str = "name: m\ndescription: d\ndecoder: semantic-mask\nparams:\n  input_size: 256\n  scale: 0.5\n  layout: nhwc\n  mean: [0.1, 2]\nclasses: [a, b]\n";

  #[test]
  fn reads_the_name_decoder_params_and_classes() {
    let manifest = Manifest::parse(TEXT).unwrap();
    assert_eq!((manifest.name(), manifest.description(), manifest.decoder()), ("m", "d", Some("semantic-mask")));
    assert_eq!(manifest.number("input_size"), Some(256.0));
    assert_eq!(manifest.number("scale"), Some(0.5));
    assert_eq!(manifest.text("layout"), Some("nhwc"));
    assert_eq!(manifest.list("mean"), Some(&[0.1, 2.0][..]));
    assert_eq!(manifest.classes(), ["a", "b"]);
    assert_eq!(manifest.number("missing"), None);
    assert!(manifest.required_number("missing").is_err());
  }

  #[test]
  fn an_image_model_manifest_keeps_its_settings_at_the_top_and_lists_its_controls() {
    let manifest = Manifest::parse(
      "name: up\nscale_factor: 2.0\ntile_size: 128\ncontrol:\n  size: 2\n  parameters:\n    - name: deblur\n      index: 0\n    - description: no name\n      index: 1\n",
    )
    .unwrap();
    assert_eq!((manifest.number("scale_factor"), manifest.number("tile_size")), (Some(2.0), Some(128.0)));
    let control = manifest.control().unwrap();
    assert_eq!(control.defaults, [0.5, 0.5], "defaults to half when none are listed");
    assert_eq!(control.parameters.len(), 1, "a parameter needs a name and an index");
    assert_eq!((control.parameters[0].name.as_str(), control.parameters[0].index), ("deblur", 0));
    assert!(Manifest::parse("name: x\ncontrol:\n  size: 0\n").unwrap().control().is_none());
  }

  #[test]
  fn a_manifest_without_a_decoder_still_parses() {
    assert_eq!(Manifest::parse("name: x\n").unwrap().decoder(), None);
  }
}
