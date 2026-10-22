use super::error::PluginError;
use saphyr::{LoadableYamlNode, Yaml};
use std::collections::{BTreeMap, HashSet};

/// A setting of a pipeline step.
#[derive(Clone, Debug, PartialEq)]
pub enum StepValue {
  Number(f64),
  Text(String),
  Bool(bool),
  /// A list of numbers, such as a color or a point.
  List(Vec<f64>),
}

/// One operation of a pipeline: the library operation to run, named as the host knows it, and its settings.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineStep {
  pub op: String,
  pub settings: BTreeMap<String, StepValue>,
}

/// An AI model a plugin adds.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelContribution {
  /// Names the model in the plugin. It is the role the AI runtime knows the model by.
  pub id: String,
  pub title: String,
  pub description: String,
  /// What a feature can ask for to find this model, such as `inpaint`.
  pub capabilities: Vec<String>,
  /// What the model finds, such as `people`, for showing it.
  pub subjects: Vec<String>,
  /// The `.onnx` file, relative to the plugin's folder.
  pub model: String,
  /// The model's manifest, relative to the plugin's folder (see `abra-ai-core`, `graph`).
  pub manifest: String,
}

/// A list of library operations a plugin adds, run in order by the host.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineContribution {
  pub id: String,
  pub title: String,
  pub description: String,
  pub capabilities: Vec<String>,
  pub steps: Vec<PipelineStep>,
}

/// A TypeScript module a plugin adds, run by the host's JavaScript engine against the same bindings the app uses.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptContribution {
  pub id: String,
  pub title: String,
  pub description: String,
  pub capabilities: Vec<String>,
  /// The module, relative to the plugin's folder.
  pub entry: String,
}

/// A file of a plugin that is downloaded to install it, and the SHA-256 it must have.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginFile {
  /// The file, relative to the plugin's folder. It is downloaded from the same place as the `plugin.yml`.
  pub path: String,
  /// Lowercase hex SHA-256 of the file.
  pub sha256: String,
}

/// A plugin that has to be installed before this one can be used.
#[derive(Clone, Debug, PartialEq)]
pub struct Dependency {
  /// The id of the plugin that is needed.
  pub id: String,
}

/// What a plugin's `plugin.yml` says about it. See the module documentation for the format.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginManifest {
  pub id: String,
  pub name: String,
  pub version: String,
  pub description: String,
  /// The files to download to install the plugin, other than the `plugin.yml`. Empty for a plugin that is already in a folder.
  pub files: Vec<PluginFile>,
  /// The plugins that have to be installed for this one to be used, so a plugin can leave out what another already has (a plugin of
  /// controls does not carry the AI models it uses). Where to get them from is not said here: the catalog knows.
  pub dependencies: Vec<Dependency>,
  pub models: Vec<ModelContribution>,
  pub pipelines: Vec<PipelineContribution>,
  pub scripts: Vec<ScriptContribution>,
}

/// Whether `p_id` is lowercase words of letters and digits joined by single dashes.
fn is_id(p_id: &str) -> bool {
  !p_id.is_empty()
    && p_id.split('-').all(|word| !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()))
}

/// Whether `p_path` stays inside the plugin's folder: relative, with `/` between names, and no `.` or `..`.
fn is_inside(p_path: &str) -> bool {
  !p_path.is_empty()
    && !p_path.contains('\\')
    && !p_path.contains(':')
    && p_path.split('/').all(|name| !name.is_empty() && name != "." && name != "..")
}

fn text<'a>(p_node: &'a Yaml, p_key: &str) -> Option<&'a str> {
  p_node.as_mapping_get(p_key).and_then(Yaml::as_str)
}

fn required<'a>(p_node: &'a Yaml, p_key: &str, p_where: &str) -> Result<&'a str, PluginError> {
  text(p_node, p_key)
    .filter(|value| !value.trim().is_empty())
    .ok_or_else(|| PluginError::invalid_manifest(format!("{p_where} needs `{p_key}`")))
}

fn texts(p_node: &Yaml, p_key: &str) -> Result<Vec<String>, PluginError> {
  let Some(list) = p_node.as_mapping_get(p_key) else {
    return Ok(Vec::new());
  };
  list
    .as_vec()
    .ok_or_else(|| PluginError::invalid_manifest(format!("`{p_key}` must be a list")))?
    .iter()
    .map(|item| {
      item.as_str().map(str::to_string).ok_or_else(|| PluginError::invalid_manifest(format!("`{p_key}` must be a list of text")))
    })
    .collect()
}

fn number(p_node: &Yaml) -> Option<f64> {
  p_node.as_floating_point().or_else(|| p_node.as_integer().map(|integer| integer as f64))
}

/// An id that follows the rules, or the reason it does not.
fn id_of(p_node: &Yaml, p_where: &str) -> Result<String, PluginError> {
  let id = required(p_node, "id", p_where)?;
  if !is_id(id) {
    return Err(PluginError::invalid_manifest(format!(
      "{p_where} has the id `{id}`; use lowercase words of letters and digits joined by dashes"
    )));
  }
  Ok(id.to_string())
}

/// What every contribution has: its id, title, description and capabilities.
fn common(p_node: &Yaml, p_kind: &str) -> Result<(String, String, String, Vec<String>), PluginError> {
  let id = id_of(p_node, &format!("A {p_kind}"))?;
  let place = format!("The {p_kind} `{id}`");
  let title = required(p_node, "title", &place)?.to_string();
  let capabilities = texts(p_node, "capabilities")?;
  if capabilities.is_empty() {
    return Err(PluginError::invalid_manifest(format!("{place} needs at least one of `capabilities`, or no feature can find it")));
  }
  Ok((id, title, text(p_node, "description").unwrap_or_default().to_string(), capabilities))
}

fn file_of(p_node: &Yaml, p_key: &str, p_where: &str) -> Result<String, PluginError> {
  let path = required(p_node, p_key, p_where)?;
  if !is_inside(path) {
    return Err(PluginError::invalid_manifest(format!(
      "{p_where} has `{p_key}: {path}`; a plugin's files are relative to its folder, with `/` between names and no `..`"
    )));
  }
  Ok(path.to_string())
}

fn entries<'a, 'y>(p_contributes: Option<&'a Yaml<'y>>, p_key: &str) -> Result<&'a [Yaml<'y>], PluginError> {
  match p_contributes.and_then(|contributes| contributes.as_mapping_get(p_key)) {
    None => Ok(&[]),
    Some(list) => list.as_vec().map(Vec::as_slice).ok_or_else(|| PluginError::invalid_manifest(format!("`{p_key}` must be a list"))),
  }
}

fn step(p_node: &Yaml, p_where: &str) -> Result<PipelineStep, PluginError> {
  let mapping =
    p_node.as_mapping().ok_or_else(|| PluginError::invalid_manifest(format!("A step of {p_where} must have an `op` and its settings")))?;
  let op = required(p_node, "op", &format!("A step of {p_where}"))?.to_string();
  let mut settings = BTreeMap::new();
  for (key, value) in mapping.iter() {
    let Some(key) = key.as_str().filter(|key| *key != "op") else { continue };
    let value = if let Some(value) = number(value) {
      StepValue::Number(value)
    } else if let Some(value) = value.as_bool() {
      StepValue::Bool(value)
    } else if let Some(value) = value.as_str() {
      StepValue::Text(value.to_string())
    } else if let Some(items) = value.as_vec() {
      StepValue::List(items.iter().filter_map(number).collect())
    } else {
      return Err(PluginError::invalid_manifest(format!("The setting `{key}` of the step `{op}` must be a number, text, true or false, or a list of numbers")));
    };
    settings.insert(key.to_string(), value);
  }
  Ok(PipelineStep { op, settings })
}

impl PluginManifest {
  /// Reads and checks a `plugin.yml`. This checks the text of the manifest only; that the files it names are there is checked when the
  /// plugin is read from its folder (see [`PluginRegistry::scan`](super::PluginRegistry::scan)).
  ///
  /// - `p_text`: The YAML of the manifest.
  pub fn parse(p_text: &str) -> Result<Self, PluginError> {
    let docs = Yaml::load_from_str(p_text).map_err(|error| PluginError::invalid_manifest(format!("not valid YAML: {error}")))?;
    let doc = docs.first().ok_or_else(|| PluginError::invalid_manifest("the manifest is empty"))?;
    let id = id_of(doc, "The plugin")?;
    let place = format!("The plugin `{id}`");
    let contributes = doc.as_mapping_get("contributes");

    let mut models = Vec::new();
    for node in entries(contributes, "models")? {
      let (id, title, description, capabilities) = common(node, "model")?;
      let place = format!("The model `{id}`");
      let model = file_of(node, "model", &place)?;
      // The AI runtime reads a model's manifest from beside it, named like the model, so that is the only place it can be.
      let expected = format!("{}.yml", model.rsplit_once('.').map_or(model.as_str(), |(stem, _)| stem));
      let manifest = match text(node, "manifest") {
        Some(_) => file_of(node, "manifest", &place)?,
        None => expected.clone(),
      };
      if manifest != expected {
        return Err(PluginError::invalid_manifest(format!(
          "{place} has `manifest: {manifest}`; the AI runtime reads a model's manifest from beside it, so it must be named `{expected}`"
        )));
      }
      models.push(ModelContribution { id, title, description, capabilities, subjects: texts(node, "subjects")?, model, manifest });
    }
    let mut pipelines = Vec::new();
    for node in entries(contributes, "pipelines")? {
      let (id, title, description, capabilities) = common(node, "pipeline")?;
      let place = format!("The pipeline `{id}`");
      let steps = node
        .as_mapping_get("steps")
        .and_then(Yaml::as_vec)
        .filter(|steps| !steps.is_empty())
        .ok_or_else(|| PluginError::invalid_manifest(format!("{place} needs `steps`")))?
        .iter()
        .map(|node| step(node, &place))
        .collect::<Result<_, _>>()?;
      pipelines.push(PipelineContribution { id, title, description, capabilities, steps });
    }
    let mut scripts = Vec::new();
    for node in entries(contributes, "scripts")? {
      let (id, title, description, capabilities) = common(node, "script")?;
      let entry = file_of(node, "entry", &format!("The script `{id}`"))?;
      scripts.push(ScriptContribution { id, title, description, capabilities, entry });
    }

    let mut dependencies: Vec<Dependency> = Vec::new();
    if let Some(list) = doc.as_mapping_get("dependencies") {
      let items = list.as_vec().ok_or_else(|| PluginError::invalid_manifest(format!("{place}: `dependencies` must be a list of `- id: plugin-id`")))?;
      for node in items {
        // Only `id` for now. Any other key is an error, so a misspelled one is not a dependency that is silently not required.
        for key in node.as_mapping().into_iter().flat_map(|mapping| mapping.iter()).filter_map(|(key, _)| key.as_str()) {
          if key != "id" {
            return Err(PluginError::invalid_manifest(format!("{place}: a dependency has `{key}`; it can only have `id`")));
          }
        }
        let needed = required(node, "id", &format!("A dependency of {place}"))?;
        if !is_id(needed) {
          return Err(PluginError::invalid_manifest(format!(
            "{place} depends on `{needed}`; use the plugin's id: lowercase words of letters and digits joined by dashes"
          )));
        }
        if needed == id {
          return Err(PluginError::invalid_manifest(format!("{place} depends on itself")));
        }
        if dependencies.iter().any(|dependency| dependency.id == needed) {
          return Err(PluginError::invalid_manifest(format!("{place} lists the dependency `{needed}` more than once")));
        }
        dependencies.push(Dependency { id: needed.to_string() });
      }
    }

    let mut files = Vec::new();
    for node in doc.as_mapping_get("files").and_then(Yaml::as_vec).into_iter().flatten() {
      let path = file_of(node, "path", &format!("{place}, in `files`,"))?;
      let sha256 = required(node, "sha256", &format!("The file `{path}` in `files`"))?.to_lowercase();
      if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PluginError::invalid_manifest(format!("The `sha256` of `{path}` is not a SHA-256")));
      }
      files.push(PluginFile { path, sha256 });
    }

    if models.is_empty() && pipelines.is_empty() && scripts.is_empty() {
      return Err(PluginError::invalid_manifest(format!("{place} adds nothing; list `models`, `pipelines` or `scripts` under `contributes`")));
    }
    let mut seen = HashSet::new();
    let ids = models.iter().map(|m| &m.id).chain(pipelines.iter().map(|p| &p.id)).chain(scripts.iter().map(|s| &s.id));
    for id in ids {
      if !seen.insert(id) {
        return Err(PluginError::invalid_manifest(format!("{place} uses the id `{id}` for more than one contribution")));
      }
    }

    Ok(Self {
      id,
      name: required(doc, "name", &place)?.to_string(),
      // A version is usually text (`1.2.0`), but `1` and `1.2` are numbers to YAML.
      version: text(doc, "version")
        .map(str::to_string)
        .or_else(|| doc.as_mapping_get("version").and_then(number).map(|version| version.to_string()))
        .filter(|version| !version.trim().is_empty())
        .ok_or_else(|| PluginError::invalid_manifest(format!("{place} needs `version`")))?,
      description: text(doc, "description").unwrap_or_default().to_string(),
      files,
      dependencies,
      models,
      pipelines,
      scripts,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FULL: &str = "id: demo-plugin\nname: Demo\nversion: 1.2.0\ndescription: d\ncontributes:\n  models:\n    - id: remover\n      title: Remover\n      capabilities: [inpaint]\n      subjects: [objects]\n      model: m/remover.onnx\n  pipelines:\n    - id: soften\n      title: Soften\n      capabilities: [effect]\n      steps:\n        - { op: gaussian_blur, radius: 12, edge: true, tint: [1, 0.5, 0], mode: soft }\n  scripts:\n    - id: collage\n      title: Collage\n      capabilities: [generator]\n      entry: collage.js\n";

  fn without(p_text: &str) -> String {
    p_text.replace("id: demo-plugin", "id: Demo Plugin")
  }

  #[test]
  fn a_plugin_lists_its_models_pipelines_and_scripts() {
    let manifest = PluginManifest::parse(FULL).unwrap();
    assert_eq!((manifest.id.as_str(), manifest.name.as_str(), manifest.version.as_str()), ("demo-plugin", "Demo", "1.2.0"));
    let model = &manifest.models[0];
    assert_eq!((model.id.as_str(), model.model.as_str()), ("remover", "m/remover.onnx"));
    assert_eq!(model.manifest, "m/remover.yml", "the model's manifest defaults to its name with .yml");
    assert_eq!((model.capabilities.clone(), model.subjects.clone()), (vec!["inpaint".to_string()], vec!["objects".to_string()]));
    let step = &manifest.pipelines[0].steps[0];
    assert_eq!(step.op, "gaussian_blur");
    assert_eq!(step.settings["radius"], StepValue::Number(12.0));
    assert_eq!(step.settings["edge"], StepValue::Bool(true));
    assert_eq!(step.settings["tint"], StepValue::List(vec![1.0, 0.5, 0.0]));
    assert_eq!(step.settings["mode"], StepValue::Text("soft".into()));
    assert_eq!(manifest.scripts[0].entry, "collage.js");
  }

  #[test]
  fn the_files_to_download_each_have_a_path_and_a_sha256() {
    let hash = "ab".repeat(32);
    let manifest = PluginManifest::parse(&format!("{FULL}files:
  - path: m/remover.onnx
    sha256: {}
", hash.to_uppercase())).unwrap();
    assert_eq!(manifest.files, vec![PluginFile { path: "m/remover.onnx".into(), sha256: hash.clone() }]);
    assert!(PluginManifest::parse(FULL).unwrap().files.is_empty());
    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&format!("{FULL}files:
  - path: ../x
    sha256: {hash}
")).contains("no `..`"));
    assert!(error(&format!("{FULL}files:
  - path: x
    sha256: abc
")).contains("not a SHA-256"));
    assert!(error(&format!("{FULL}files:
  - path: x
")).contains("sha256"));
  }

  #[test]
  fn what_a_plugin_depends_on_is_a_list_of_plugin_ids() {
    let with = |dependencies: &str| format!("{FULL}dependencies:\n{dependencies}");
    let manifest = PluginManifest::parse(&with("  - id: ai-face-landmarks\n  - id: ai-face-parsing\n")).unwrap();
    let ids: Vec<&str> = manifest.dependencies.iter().map(|dependency| dependency.id.as_str()).collect();
    assert_eq!(ids, ["ai-face-landmarks", "ai-face-parsing"]);
    assert!(PluginManifest::parse(FULL).unwrap().dependencies.is_empty(), "a plugin needs nothing unless it says so");

    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&with("  - ids: x\n")).contains("can only have `id`"), "a misspelled key is an error");
    assert!(error(&with("  - id: Not An Id\n")).contains("use the plugin's id"));
    assert!(error(&with("  - id: demo-plugin\n")).contains("depends on itself"));
    assert!(error(&with("  - id: other\n  - id: other\n")).contains("more than once"));
    assert!(error(&with("  - title: x\n")).contains("can only have `id`"));
    assert!(error(&with("  id: other\n")).contains("must be a list"));
  }

  #[test]
  fn a_manifest_that_is_wrong_says_what_is_wrong() {
    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&without(FULL)).contains("Demo Plugin"), "an id is lowercase words with dashes");
    assert!(error(&FULL.replace("version: 1.2.0\n", "")).contains("version"));
    assert!(error(&FULL.replace("capabilities: [inpaint]", "capabilities: []")).contains("capabilities"));
    assert!(error(&FULL.replace("model: m/remover.onnx", "model: ../remover.onnx")).contains("no `..`"));
    assert!(error(&FULL.replace("      model: m/remover.onnx\n", "      model: m/remover.onnx\n      manifest: m/other.yml\n")).contains("must be named `m/remover.yml`"));
    assert!(error(&FULL.replace("entry: collage.js", "entry: /sdcard/collage.js")).contains("collage.js"));
    assert!(error(&FULL.replace("id: collage", "id: soften")).contains("more than one"));
    assert!(error("id: empty\nname: E\nversion: 1\n").contains("adds nothing"));
    assert!(error("id: a\nname: [").contains("not valid YAML"));
    assert!(error(&FULL.replace("        - { op: gaussian_blur, radius: 12, edge: true, tint: [1, 0.5, 0], mode: soft }\n", "        - { radius: 12 }\n")).contains("op"));
  }
}
