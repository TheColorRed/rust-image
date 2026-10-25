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

/// A field a plugin asks the host to show in its settings form.
#[derive(Clone, Debug, PartialEq)]
pub enum PluginSetting {
  Text {
    id: String,
    label: String,
    description: Option<String>,
    placeholder: Option<String>,
    default: Option<String>,
    required: bool,
  },
  Option {
    id: String,
    label: String,
    description: Option<String>,
    options: Vec<String>,
    default: Option<String>,
    required: bool,
  },
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

/// A control a plugin adds to the editor. Its place is its `root` and then its `path`, and its script makes the control. A plugin can
/// have controls under more than one root.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlContribution {
  /// The tab and groups the control goes in, as the names a person sees, separated by `/`, like `Beauty/Makeup`. A tab or group that is
  /// not there is made.
  pub root: String,
  /// Where the control goes under the root: its name, which is what people see, or the names of groups and then its name
  /// (`Eyes/Eyeliner`), separated by `/`.
  pub path: String,
  /// The control's script, relative to the plugin's folder. Its default export is a function that returns the control.
  pub entry: String,
}

impl ControlContribution {
  /// The control's name: the last name in its path, which is what people see.
  pub fn name(&self) -> &str {
    self.path.rsplit('/').next().unwrap_or(&self.path)
  }
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
  pub subscription: bool,
  /// Optional settings form declared by the plugin, in display order.
  pub settings: Vec<PluginSetting>,
  /// The files to download to install the plugin, other than the `plugin.yml`. Empty for a plugin that is already in a folder.
  pub files: Vec<PluginFile>,
  /// The plugins that have to be installed for this one to be used, so a plugin can leave out what another already has (a plugin of
  /// controls does not carry the AI models it uses). Where to get them from is not said here: the catalog knows.
  pub dependencies: Vec<Dependency>,
  pub models: Vec<ModelContribution>,
  pub pipelines: Vec<PipelineContribution>,
  pub scripts: Vec<ScriptContribution>,
  /// The controls the plugin adds, under one root or several, in the order they go in their place.
  pub controls: Vec<ControlContribution>,
}

/// Whether `p_id` is a lowercase author and plugin name, separated by a dot, with words joined by single dashes.
fn is_id(p_id: &str) -> bool {
  fn is_name(p_name: &str) -> bool {
    !p_name.is_empty()
      && p_name
        .split('-')
        .all(|word| !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()))
  }

  let mut parts = p_id.split('.');
  matches!((parts.next(), parts.next(), parts.next()), (Some(author), Some(name), None) if is_name(author) && is_name(name))
}

/// Whether `p_id` is a contribution id scoped by its plugin.
fn is_contribution_id(p_id: &str) -> bool {
  !p_id.is_empty()
    && p_id
      .split('-')
      .all(|word| !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()))
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

fn boolean(p_node: &Yaml, p_key: &str) -> Option<bool> {
  p_node.as_mapping_get(p_key).and_then(Yaml::as_bool)
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
      item
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| PluginError::invalid_manifest(format!("`{p_key}` must be a list of text")))
    })
    .collect()
}

fn number(p_node: &Yaml) -> Option<f64> {
  p_node.as_floating_point().or_else(|| p_node.as_integer().map(|integer| integer as f64))
}

fn optional_text(p_node: &Yaml, p_key: &str, p_where: &str) -> Result<Option<String>, PluginError> {
  p_node
    .as_mapping_get(p_key)
    .map(|value| {
      value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| PluginError::invalid_manifest(format!("{p_where}: `{p_key}` must be text")))
    })
    .transpose()
}

fn plugin_settings(p_node: &Yaml) -> Result<Vec<PluginSetting>, PluginError> {
  let Some(settings) = p_node.as_mapping_get("settings") else {
    return Ok(Vec::new());
  };
  let entries =
    settings.as_vec().ok_or_else(|| PluginError::invalid_manifest("`settings` must be a list of setting fields"))?;
  let mut parsed = Vec::with_capacity(entries.len());
  let mut ids = HashSet::new();
  for node in entries {
    let id = id_of(node, "A plugin setting")?;
    if !ids.insert(id.clone()) {
      return Err(PluginError::invalid_manifest(format!("The setting `{id}` is listed more than once")));
    }
    let place = format!("The setting `{id}`");
    let attributes = node
      .as_mapping_get("attributes")
      .filter(|attributes| attributes.as_mapping().is_some())
      .ok_or_else(|| PluginError::invalid_manifest(format!("{place} needs an `attributes` map")))?;
    let label = required(attributes, "label", &place)?.to_string();
    let description = optional_text(attributes, "description", &place)?;
    let validations = node.as_mapping_get("validations");
    if validations.is_some_and(|validations| validations.as_mapping().is_none()) {
      return Err(PluginError::invalid_manifest(format!("{place}: `validations` must be a map")));
    }
    let is_required = validations
      .and_then(|validations| validations.as_mapping_get("required"))
      .map(|value| {
        value.as_bool().ok_or_else(|| {
          PluginError::invalid_manifest(format!("{place}: `validations.required` must be true or false"))
        })
      })
      .transpose()?
      .unwrap_or(false);
    let setting = match required(node, "type", &place)? {
      "text" => {
        if attributes.as_mapping_get("options").is_some() {
          return Err(PluginError::invalid_manifest(format!("{place}: `text` does not take `attributes.options`")));
        }
        PluginSetting::Text {
          id,
          label,
          description,
          placeholder: optional_text(attributes, "placeholder", &place)?,
          default: optional_text(attributes, "value", &place)?,
          required: is_required,
        }
      }
      "option" => {
        if attributes.as_mapping_get("placeholder").is_some() || attributes.as_mapping_get("value").is_some() {
          return Err(PluginError::invalid_manifest(format!(
            "{place}: `option` does not take `placeholder` or `value`"
          )));
        }
        let options = attributes
          .as_mapping_get("options")
          .and_then(Yaml::as_vec)
          .filter(|options| !options.is_empty())
          .ok_or_else(|| PluginError::invalid_manifest(format!("{place}: `option` needs a non-empty `options` list")))?
          .iter()
          .map(|option| {
            option
              .as_str()
              .filter(|option| !option.trim().is_empty())
              .map(str::to_string)
              .ok_or_else(|| PluginError::invalid_manifest(format!("{place}: `options` must be non-empty text values")))
          })
          .collect::<Result<Vec<_>, _>>()?;
        let default = optional_text(attributes, "default", &place)?;
        if let Some(default) = &default {
          if !options.contains(default) {
            return Err(PluginError::invalid_manifest(format!("{place}: `default` must match one of its `options`")));
          }
        }
        PluginSetting::Option {
          id,
          label,
          description,
          options,
          default,
          required: is_required,
        }
      }
      kind => {
        return Err(PluginError::invalid_manifest(format!(
          "{place} has unknown type `{kind}`; expected `text` or `option`"
        )))
      }
    };
    parsed.push(setting);
  }
  Ok(parsed)
}

/// An id that follows the rules, or the reason it does not.
fn id_of(p_node: &Yaml, p_where: &str) -> Result<String, PluginError> {
  let id = required(p_node, "id", p_where)?;
  let valid = if p_where == "The plugin" { is_id(id) } else { is_contribution_id(id) };
  if !valid {
    return Err(PluginError::invalid_manifest(format!(
      "{p_where} has the id `{id}`; use {}",
      if p_where == "The plugin" {
        "`<author>.<id>` with lowercase words of letters and digits joined by dashes"
      } else {
        "lowercase words of letters and digits joined by dashes"
      }
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
    return Err(PluginError::invalid_manifest(format!(
      "{place} needs at least one of `capabilities`, or no feature can find it"
    )));
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
    Some(list) => {
      list.as_vec().map(Vec::as_slice).ok_or_else(|| PluginError::invalid_manifest(format!("`{p_key}` must be a list")))
    }
  }
}

fn step(p_node: &Yaml, p_where: &str) -> Result<PipelineStep, PluginError> {
  let mapping = p_node
    .as_mapping()
    .ok_or_else(|| PluginError::invalid_manifest(format!("A step of {p_where} must have an `op` and its settings")))?;
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
      return Err(PluginError::invalid_manifest(format!(
        "The setting `{key}` of the step `{op}` must be a number, text, true or false, or a list of numbers"
      )));
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
    let docs =
      Yaml::load_from_str(p_text).map_err(|error| PluginError::invalid_manifest(format!("not valid YAML: {error}")))?;
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
      models.push(ModelContribution {
        id,
        title,
        description,
        capabilities,
        subjects: texts(node, "subjects")?,
        model,
        manifest,
      });
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
      pipelines.push(PipelineContribution {
        id,
        title,
        description,
        capabilities,
        steps,
      });
    }
    let mut scripts = Vec::new();
    for node in entries(contributes, "scripts")? {
      let (id, title, description, capabilities) = common(node, "script")?;
      let entry = file_of(node, "entry", &format!("The script `{id}`"))?;
      scripts.push(ScriptContribution {
        id,
        title,
        description,
        capabilities,
        entry,
      });
    }


    let mut dependencies: Vec<Dependency> = Vec::new();
    if let Some(list) = doc.as_mapping_get("dependencies") {
      let items = list.as_vec().ok_or_else(|| {
        PluginError::invalid_manifest(format!("{place}: `dependencies` must be a list of `- id: plugin-id`"))
      })?;
      for node in items {
        // Only `id` for now. Any other key is an error, so a misspelled one is not a dependency that is silently not required.
        for key in node.as_mapping().into_iter().flat_map(|mapping| mapping.iter()).filter_map(|(key, _)| key.as_str())
        {
          if key != "id" {
            return Err(PluginError::invalid_manifest(format!(
              "{place}: a dependency has `{key}`; it can only have `id`"
            )));
          }
        }
        let needed = required(node, "id", &format!("A dependency of {place}"))?;
        if !is_id(needed) {
          return Err(PluginError::invalid_manifest(format!(
            "{place} depends on `{needed}`; use the plugin's id: `<author>.<id>` with lowercase words of letters and digits joined by dashes"
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

    // The controls, in groups that each have a root. The names are matched to what people see, ignoring case, so two paths that differ
    // only by case are the same control.
    let names = |path: &str, what: &str, example: &str| -> Result<String, PluginError> {
      if path.split('/').any(|name| name.trim().is_empty()) {
        return Err(PluginError::invalid_manifest(format!(
          "{place} has {what} `{path}`; write names separated by `/`, none empty, like `{example}`"
        )));
      }
      Ok(path.split('/').map(str::trim).collect::<Vec<_>>().join("/"))
    };
    let mut controls: Vec<ControlContribution> = Vec::new();
    for group in entries(contributes, "controls")? {
      let root =
        names(required(group, "root", &format!("A group of controls of {place}"))?, "`root:`", "Beauty/Makeup")?;
      let items =
        group.as_mapping_get("items").and_then(Yaml::as_vec).filter(|items| !items.is_empty()).ok_or_else(|| {
          PluginError::invalid_manifest(format!("{place}: the controls under `root: {root}` need `items`"))
        })?;
      for node in items {
        let path = names(
          required(node, "path", &format!("A control under `root: {root}` of {place}"))?,
          "the control path",
          "Lipstick",
        )?;
        let entry = file_of(node, "entry", &format!("The control `{path}` under `root: {root}`"))?;
        if controls
          .iter()
          .any(|control| control.root.eq_ignore_ascii_case(&root) && control.path.eq_ignore_ascii_case(&path))
        {
          return Err(PluginError::invalid_manifest(format!(
            "{place} has the control `{path}` under `root: {root}` more than once"
          )));
        }
        controls.push(ControlContribution {
          root: root.clone(),
          path,
          entry,
        });
      }
    }

    if models.is_empty() && pipelines.is_empty() && scripts.is_empty() && controls.is_empty() {
      return Err(PluginError::invalid_manifest(format!(
        "{place} adds nothing; list `models`, `pipelines`, `scripts` or `controls` under `contributes`"
      )));
    }
    let mut seen = HashSet::new();
    let ids = models.iter().map(|m| &m.id).chain(pipelines.iter().map(|p| &p.id)).chain(scripts.iter().map(|s| &s.id));
    for id in ids {
      if !seen.insert(id) {
        return Err(PluginError::invalid_manifest(format!(
          "{place} uses the id `{id}` for more than one contribution"
        )));
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
      subscription: boolean(doc, "subscription").unwrap_or(false),
      settings: plugin_settings(doc)?,
      files,
      dependencies,
      models,
      pipelines,
      scripts,
      controls,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FULL: &str = "id: demo.author\nname: Demo\nversion: 1.2.0\ndescription: d\ncontributes:\n  models:\n    - id: remover\n      title: Remover\n      capabilities: [inpaint]\n      subjects: [objects]\n      model: m/remover.onnx\n  pipelines:\n    - id: soften\n      title: Soften\n      capabilities: [effect]\n      steps:\n        - { op: gaussian_blur, radius: 12, edge: true, tint: [1, 0.5, 0], mode: soft }\n  scripts:\n    - id: collage\n      title: Collage\n      capabilities: [generator]\n      entry: collage.js\n";

  fn without(p_text: &str) -> String {
    p_text.replace("id: demo.author", "id: Demo Plugin")
  }

  #[test]
  fn a_plugin_lists_its_models_pipelines_and_scripts() {
    let manifest = PluginManifest::parse(FULL).unwrap();
    assert_eq!(
      (manifest.id.as_str(), manifest.name.as_str(), manifest.version.as_str()),
      ("demo.author", "Demo", "1.2.0")
    );
    let model = &manifest.models[0];
    assert_eq!((model.id.as_str(), model.model.as_str()), ("remover", "m/remover.onnx"));
    assert_eq!(model.manifest, "m/remover.yml", "the model's manifest defaults to its name with .yml");
    assert_eq!(
      (model.capabilities.clone(), model.subjects.clone()),
      (vec!["inpaint".to_string()], vec!["objects".to_string()])
    );
    let step = &manifest.pipelines[0].steps[0];
    assert_eq!(step.op, "gaussian_blur");
    assert_eq!(step.settings["radius"], StepValue::Number(12.0));
    assert_eq!(step.settings["edge"], StepValue::Bool(true));
    assert_eq!(step.settings["tint"], StepValue::List(vec![1.0, 0.5, 0.0]));
    assert_eq!(step.settings["mode"], StepValue::Text("soft".into()));
    assert_eq!(manifest.scripts[0].entry, "collage.js");
  }

  #[test]
  fn a_plugin_lists_its_controls_under_one_root_or_several() {
    let with = |controls: &str| format!("id: demo.pack\nname: Pack\nversion: 1.0.0\ncontributes:\n  controls:\n{controls}");
    let ok = PluginManifest::parse(&with(
      "    - root: Beauty / Makeup\n      items:\n        - path: Lipstick\n          entry: lipstick.js\n        - path: Eyes/ Eyeliner\n          entry: s/eyeliner.js\n    - root: Beauty/Hair\n      items:\n        - path: Color\n          entry: hair.js\n",
    ))
    .unwrap();
    let controls: Vec<(&str, &str, &str, &str)> = ok
      .controls
      .iter()
      .map(|control| (control.root.as_str(), control.path.as_str(), control.name(), control.entry.as_str()))
      .collect();
    assert_eq!(
      controls,
      [
        ("Beauty/Makeup", "Lipstick", "Lipstick", "lipstick.js"),
        ("Beauty/Makeup", "Eyes/Eyeliner", "Eyeliner", "s/eyeliner.js"),
        ("Beauty/Hair", "Color", "Color", "hair.js"),
      ],
      "spaces round the names go, and the order is the order written"
    );
    assert!(PluginManifest::parse(FULL).unwrap().controls.is_empty());

    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&with("    - items:\n        - path: A\n          entry: a.js\n")).contains("needs `root`"));
    assert!(error(&with("    - root: Beauty\n")).contains("need `items`"));
    assert!(error(&with("    - root: Beauty\n      items: []\n")).contains("need `items`"));
    for wrong in ["/Beauty", "Beauty/", "Beauty//Makeup"] {
      let text = with(&format!("    - root: {wrong}\n      items:\n        - path: A\n          entry: a.js\n"));
      assert!(error(&text).contains("like `Beauty/Makeup`"), "{wrong}");
    }
    let one = |item: &str| with(&format!("    - root: Beauty\n      items:\n{item}"));
    assert!(error(&one("        - path: A//B\n          entry: a.js\n")).contains("like `Lipstick`"));
    assert!(error(&one("        - entry: a.js\n")).contains("needs `path`"));
    assert!(error(&one("        - path: A\n")).contains("needs `entry`"));
    assert!(error(&one("        - path: A\n          entry: ../a.js\n")).contains("no `..`"));
    assert!(error(&one(
      "        - path: Lipstick\n          entry: a.js\n        - path: lipstick\n          entry: b.js\n"
    ))
    .contains("more than once"));
    // The same name under another root is another control.
    let twice = with(
      "    - root: Beauty\n      items:\n        - path: Color\n          entry: a.js\n    - root: Other\n      items:\n        - path: Color\n          entry: b.js\n",
    );
    assert_eq!(PluginManifest::parse(&twice).unwrap().controls.len(), 2);
  }

  #[test]
  fn the_files_to_download_each_have_a_path_and_a_sha256() {
    let hash = "ab".repeat(32);
    let manifest =
      PluginManifest::parse(&format!("{FULL}files:\n  - path: m/remover.onnx\n    sha256: {}\n", hash.to_uppercase()))
        .unwrap();
    assert_eq!(
      manifest.files,
      vec![PluginFile {
        path: "m/remover.onnx".into(),
        sha256: hash.clone()
      }]
    );
    assert!(PluginManifest::parse(FULL).unwrap().files.is_empty());
    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&format!("{FULL}files:\n  - path: ../x\n    sha256: {hash}\n")).contains("no `..`"));
    assert!(error(&format!("{FULL}files:\n  - path: x\n    sha256: abc\n")).contains("not a SHA-256"));
    assert!(error(&format!("{FULL}files:\n  - path: x\n")).contains("sha256"));
  }

  #[test]
  fn what_a_plugin_depends_on_is_a_list_of_plugin_ids() {
    let with = |dependencies: &str| format!("{FULL}dependencies:\n{dependencies}");
    let manifest = PluginManifest::parse(&with("  - id: alakazam.face-landmarks\n  - id: alakazam.face-parsing\n")).unwrap();
    let ids: Vec<&str> = manifest.dependencies.iter().map(|dependency| dependency.id.as_str()).collect();
    assert_eq!(ids, ["alakazam.face-landmarks", "alakazam.face-parsing"]);
    assert!(PluginManifest::parse(FULL).unwrap().dependencies.is_empty(), "a plugin needs nothing unless it says so");

    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&with("  - ids: x\n")).contains("can only have `id`"), "a misspelled key is an error");
    assert!(error(&with("  - id: Not An Id\n")).contains("use the plugin's id"));
    assert!(error(&with("  - id: demo.author\n")).contains("depends on itself"));
    assert!(error(&with("  - id: other.plugin\n  - id: other.plugin\n")).contains("more than once"));
    assert!(error(&with("  - title: x\n")).contains("can only have `id`"));
    assert!(error(&with("  id: other.plugin\n")).contains("must be a list"));
    for invalid in ["demo-plugin", ".plugin", "author.", "one.two.three"] {
      assert!(error(&with(&format!("  - id: {invalid}\n"))).contains("use the plugin's id"), "{invalid}");
    }
  }

  #[test]
  fn a_manifest_that_is_wrong_says_what_is_wrong() {
    let error = |text: &str| PluginManifest::parse(text).unwrap_err().to_string();
    assert!(error(&without(FULL)).contains("Demo Plugin"), "an id is an author and name separated by a dot");
    assert!(error(&FULL.replace("version: 1.2.0\n", "")).contains("version"));
    assert!(error(&FULL.replace("capabilities: [inpaint]", "capabilities: []")).contains("capabilities"));
    assert!(error(&FULL.replace("model: m/remover.onnx", "model: ../remover.onnx")).contains("no `..`"));
    assert!(error(
      &FULL.replace("      model: m/remover.onnx\n", "      model: m/remover.onnx\n      manifest: m/other.yml\n")
    )
    .contains("must be named `m/remover.yml`"));
    assert!(error(&FULL.replace("entry: collage.js", "entry: /sdcard/collage.js")).contains("collage.js"));
    assert!(error(&FULL.replace("id: collage", "id: soften")).contains("more than one"));
    assert!(error("id: alakazam.empty\nname: E\nversion: 1\n").contains("adds nothing"));
    assert!(error("id: a\nname: [").contains("not valid YAML"));
    assert!(error(&FULL.replace(
      "        - { op: gaussian_blur, radius: 12, edge: true, tint: [1, 0.5, 0], mode: soft }\n",
      "        - { radius: 12 }\n"
    ))
    .contains("op"));
  }
}
