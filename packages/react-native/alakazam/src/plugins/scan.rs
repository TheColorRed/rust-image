//! Runtime plugins: `scan_plugins(folder)` reads every plugin in a folder, as `PluginRegistry::scan` does.
//!
//! A plugin is a folder with a `plugin.yml` (see [`super`]). What a plugin adds comes back as records, with the paths of its files
//! made absolute, so the host can load a model with `AiModel.load`, run a script, or map a pipeline's steps to the library. Nothing here
//! runs a plugin. A plugin whose dependencies are not installed is listed with `missing_dependencies`, so the host can look them up in its
//! catalog and offer to install them.

use std::collections::HashMap;

use super::{Plugin, PluginRegistry, StepValue as Value};

/// A setting of a pipeline step.
#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum StepValue {
  Number(f64),
  Text(String),
  Bool(bool),
  /// A list of numbers, such as a color or a point.
  List(Vec<f64>),
}

/// One operation of a pipeline: the library operation to run and its settings.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PipelineStep {
  pub op: String,
  pub settings: HashMap<String, StepValue>,
}

/// An AI model a plugin adds.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginModel {
  pub id: String,
  pub title: String,
  pub description: String,
  pub capabilities: Vec<String>,
  pub subjects: Vec<String>,
  /// The `.onnx` file, as a path on this device.
  pub model_path: String,
  /// The model's manifest, as a path on this device.
  pub manifest_path: String,
}

/// A list of library operations a plugin adds.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginPipeline {
  pub id: String,
  pub title: String,
  pub description: String,
  pub capabilities: Vec<String>,
  pub steps: Vec<PipelineStep>,
}

/// A TypeScript module a plugin adds.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginScript {
  pub id: String,
  pub title: String,
  pub description: String,
  pub capabilities: Vec<String>,
  /// The module, as a path on this device.
  pub entry_path: String,
}

/// A control a plugin adds to the editor.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginControl {
  /// The tab and groups it goes in, as the names a person sees, separated by `/`, like `Beauty/Makeup`.
  pub root: String,
  /// Where it goes under the root: its name, or the names of groups and then its name, separated by `/`.
  pub path: String,
  /// The control's script, as a path on this device. Its default export is a function that returns the control.
  pub entry_path: String,
}

/// A full-screen plugin editor workspace.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginWorkspace {
  pub id: String,
  pub title: String,
  pub description: String,
  /// The workspace module, as a path on this device.
  pub entry_path: String,
}

/// A plugin found in the folder, and everything it adds.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginInfo {
  pub id: String,
  pub name: String,
  pub version: String,
  pub description: String,
  /// The plugin's folder, as a path on this device.
  pub dir: String,
  /// The ids of the plugins this one depends on.
  pub dependencies: Vec<String>,
  /// The ids of the plugins it depends on that cannot be used, so it adds nothing until they are installed. Empty when it can be used.
  pub missing_dependencies: Vec<String>,
  pub models: Vec<PluginModel>,
  pub pipelines: Vec<PluginPipeline>,
  pub scripts: Vec<PluginScript>,
  pub workspaces: Vec<PluginWorkspace>,
  /// The controls the plugin adds, under one root or several, in the order they go in their place.
  pub controls: Vec<PluginControl>,
}

/// A plugin that was in the folder but could not be used.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginProblem {
  pub dir: String,
  /// Why, in words for a person.
  pub message: String,
}

/// What a folder of plugins holds: the plugins that can be used, and the ones that could not.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PluginScan {
  pub plugins: Vec<PluginInfo>,
  pub problems: Vec<PluginProblem>,
}

fn path(p_plugin: &Plugin, p_relative: &str) -> String {
  p_plugin.path(p_relative).to_string_lossy().into_owned()
}

fn value(p_value: &Value) -> StepValue {
  match p_value {
    Value::Number(number) => StepValue::Number(*number),
    Value::Text(text) => StepValue::Text(text.clone()),
    Value::Bool(flag) => StepValue::Bool(*flag),
    Value::List(items) => StepValue::List(items.clone()),
  }
}

fn info(p_plugin: &Plugin, p_registry: &PluginRegistry) -> PluginInfo {
  let manifest = &p_plugin.manifest;
  PluginInfo {
    id: manifest.id.clone(),
    name: manifest.name.clone(),
    version: manifest.version.clone(),
    description: manifest.description.clone(),
    dir: p_plugin.dir.to_string_lossy().into_owned(),
    dependencies: manifest.dependencies.iter().map(|dependency| dependency.id.clone()).collect(),
    missing_dependencies: p_registry.missing_dependencies(p_plugin),
    models: manifest
      .models
      .iter()
      .map(|model| PluginModel {
        id: model.id.clone(),
        title: model.title.clone(),
        description: model.description.clone(),
        capabilities: model.capabilities.clone(),
        subjects: model.subjects.clone(),
        model_path: path(p_plugin, &model.model),
        manifest_path: path(p_plugin, &model.manifest),
      })
      .collect(),
    pipelines: manifest
      .pipelines
      .iter()
      .map(|pipeline| PluginPipeline {
        id: pipeline.id.clone(),
        title: pipeline.title.clone(),
        description: pipeline.description.clone(),
        capabilities: pipeline.capabilities.clone(),
        steps: pipeline
          .steps
          .iter()
          .map(|step| PipelineStep {
            op: step.op.clone(),
            settings: step.settings.iter().map(|(key, setting)| (key.clone(), value(setting))).collect(),
          })
          .collect(),
      })
      .collect(),
    scripts: manifest
      .scripts
      .iter()
      .map(|script| PluginScript {
        id: script.id.clone(),
        title: script.title.clone(),
        description: script.description.clone(),
        capabilities: script.capabilities.clone(),
        entry_path: path(p_plugin, &script.entry),
      })
      .collect(),
    workspaces: manifest
      .workspaces
      .iter()
      .map(|workspace| PluginWorkspace {
        id: workspace.id.clone(),
        title: workspace.title.clone(),
        description: workspace.description.clone(),
        entry_path: path(p_plugin, &workspace.entry),
      })
      .collect(),
    controls: manifest
      .controls
      .iter()
      .map(|control| PluginControl {
        root: control.root.clone(),
        path: control.path.clone(),
        entry_path: path(p_plugin, &control.entry),
      })
      .collect(),
  }
}

/// Reads every plugin in a folder: each folder in it that has a `plugin.yml`. A plugin that cannot be used is in `problems` and does not
/// stop the others. A folder that is not there holds no plugins.
#[uniffi::export]
pub fn scan_plugins(folder: String) -> PluginScan {
  let registry = PluginRegistry::scan(folder);
  PluginScan {
    plugins: registry.plugins().iter().map(|plugin| info(plugin, &registry)).collect(),
    problems: registry
      .problems()
      .iter()
      .map(|problem| PluginProblem {
        dir: problem.dir.to_string_lossy().into_owned(),
        message: problem.error.to_string(),
      })
      .collect(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_folder_of_plugins_is_scanned_through_the_bridge() {
    let folder = std::env::temp_dir().join(format!("abra-uniffi-plugins-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let plugin = folder.join("remover");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(plugin.join("m.onnx"), b"x").unwrap();
    std::fs::write(plugin.join("m.yml"), b"x").unwrap();
    std::fs::write(
      plugin.join("plugin.yml"),
      "id: test.remover\nname: Remover\nversion: 1.0.0\ncontributes:\n  models:\n    - id: remover-model\n      title: Remover\n      capabilities: [inpaint]\n      model: m.onnx\n  pipelines:\n    - id: soften\n      title: Soften\n      capabilities: [effect]\n      steps:\n        - { op: gaussian_blur, radius: 4 }\n",
    )
    .unwrap();
    // A plugin that needs one that is installed and one that is not.
    let beauty = folder.join("beauty");
    std::fs::create_dir_all(&beauty).unwrap();
    std::fs::write(beauty.join("s.js"), b"x").unwrap();
    std::fs::write(
      beauty.join("plugin.yml"),
      "id: test.beauty
name: Beauty
version: 1.0.0
dependencies:
  - id: test.remover
  - id: test.ghost
contributes:
  scripts:
    - id: run
      title: Run
      capabilities: [beauty]
      entry: s.js
",
    )
    .unwrap();
    // A plugin that adds controls under two roots.
    let pack = folder.join("pack");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(pack.join("a.js"), b"x").unwrap();
    std::fs::write(pack.join("b.js"), b"x").unwrap();
    std::fs::write(
      pack.join("plugin.yml"),
      "id: test.pack
name: Pack
version: 1.0.0
contributes:
  controls:
    - root: Beauty/Makeup
      items:
        - path: Lipstick
          entry: a.js
    - root: Beauty/Hair
      items:
        - path: Eyes/Color
          entry: b.js
",
    )
    .unwrap();
    let broken = folder.join("broken");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(broken.join("plugin.yml"), "id: [").unwrap();

    let scan = scan_plugins(folder.to_string_lossy().into_owned());
    let _ = std::fs::remove_dir_all(&folder);
    assert_eq!(scan.plugins.len(), 3);
    assert_eq!(scan.problems.len(), 1);
    let found = scan.plugins.iter().find(|plugin| plugin.id == "test.remover").unwrap();
    assert!(found.dependencies.is_empty() && found.missing_dependencies.is_empty());
    let needy = scan.plugins.iter().find(|plugin| plugin.id == "test.beauty").unwrap();
    assert_eq!(needy.dependencies, ["test.remover", "test.ghost"]);
    assert_eq!(needy.missing_dependencies, ["test.ghost"], "what is installed is not missing");
    let pack = scan.plugins.iter().find(|plugin| plugin.id == "test.pack").unwrap();
    let controls: Vec<(&str, &str)> = pack.controls.iter().map(|control| (control.root.as_str(), control.path.as_str())).collect();
    assert_eq!(controls, [("Beauty/Makeup", "Lipstick"), ("Beauty/Hair", "Eyes/Color")]);
    assert!(pack.controls[0].entry_path.ends_with("a.js") && pack.controls[0].entry_path.starts_with(&pack.dir));
    assert_eq!((found.id.as_str(), found.models[0].capabilities.clone()), ("test.remover", vec!["inpaint".to_string()]));
    assert!(found.models[0].model_path.ends_with("m.onnx") && found.models[0].model_path.starts_with(&found.dir));
    assert!(found.models[0].manifest_path.ends_with("m.yml"));
    assert_eq!(found.pipelines[0].steps[0].settings["radius"], StepValue::Number(4.0));
  }
}
