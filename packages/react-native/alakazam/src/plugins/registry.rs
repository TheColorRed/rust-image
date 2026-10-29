use super::{
  error::PluginError,
  manifest::{ControlContribution, ModelContribution, PipelineContribution, PluginManifest, ScriptContribution, WorkspaceContribution},
};
use std::{
  collections::HashSet,
  path::{Path, PathBuf},
};

/// A plugin found in a folder: what it says about itself, and where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Plugin {
  pub manifest: PluginManifest,
  /// The plugin's folder.
  pub dir: PathBuf,
}

impl Plugin {
  /// A file of the plugin, from its path relative to the plugin's folder, such as a contribution's `model` or `entry`.
  /// - `p_relative`: The path as the manifest wrote it.
  pub fn path(&self, p_relative: &str) -> PathBuf {
    self.dir.join(p_relative)
  }

  /// Reads a plugin from its folder: the `plugin.yml` in it, and a check that every file it names is there.
  ///
  /// - `p_dir`: The plugin's folder.
  pub fn read(p_dir: impl AsRef<Path>) -> Result<Self, PluginError> {
    let dir = p_dir.as_ref().to_path_buf();
    let text = std::fs::read_to_string(dir.join("plugin.yml"))
      .map_err(|error| PluginError::io(format!("{} has no readable plugin.yml ({error})", dir.display())))?;
    let manifest = PluginManifest::parse(&text)?;
    let plugin = Self { manifest, dir };
    let manifest = &plugin.manifest;
    let files = manifest
      .models
      .iter()
      .flat_map(|model| [&model.model, &model.manifest])
      .chain(manifest.scripts.iter().map(|script| &script.entry))
      .chain(manifest.workspaces.iter().map(|workspace| &workspace.entry))
      .chain(manifest.controls.iter().map(|control| &control.entry));
    for file in files {
      if !plugin.path(file).is_file() {
        return Err(PluginError::file_not_found(file.clone()));
      }
    }
    Ok(plugin)
  }
}

/// A plugin that was in the folder but could not be used, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginProblem {
  pub dir: PathBuf,
  pub error: PluginError,
}

/// One thing a plugin adds, with the plugin it came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Contribution<'a> {
  Model(&'a Plugin, &'a ModelContribution),
  Pipeline(&'a Plugin, &'a PipelineContribution),
  Script(&'a Plugin, &'a ScriptContribution),
  /// A control for the editor. It has no capabilities, because it is not found by asking for one: it is placed by its plugin's `root`.
  Control(&'a Plugin, &'a ControlContribution),
  /// A full-screen editor workspace with plugin-defined behavior.
  Workspace(&'a Plugin, &'a WorkspaceContribution),
}

impl<'a> Contribution<'a> {
  /// The plugin this came from.
  pub fn plugin(&self) -> &'a Plugin {
    match self {
      Contribution::Model(plugin, _)
      | Contribution::Pipeline(plugin, _)
      | Contribution::Script(plugin, _)
      | Contribution::Control(plugin, _)
      | Contribution::Workspace(plugin, _) => plugin,
    }
  }

  /// Its id in the plugin.
  pub fn id(&self) -> &'a str {
    match self {
      Contribution::Model(_, model) => &model.id,
      Contribution::Pipeline(_, pipeline) => &pipeline.id,
      Contribution::Script(_, script) => &script.id,
      Contribution::Control(_, control) => &control.path,
      Contribution::Workspace(_, workspace) => &workspace.id,
    }
  }

  /// The name to show for it.
  pub fn title(&self) -> &'a str {
    match self {
      Contribution::Model(_, model) => &model.title,
      Contribution::Pipeline(_, pipeline) => &pipeline.title,
      Contribution::Script(_, script) => &script.title,
      Contribution::Control(_, control) => control.name(),
      Contribution::Workspace(_, workspace) => &workspace.title,
    }
  }

  /// What a feature can ask for to find it.
  pub fn capabilities(&self) -> &'a [String] {
    match self {
      Contribution::Model(_, model) => &model.capabilities,
      Contribution::Pipeline(_, pipeline) => &pipeline.capabilities,
      Contribution::Script(_, script) => &script.capabilities,
      Contribution::Control(..) | Contribution::Workspace(..) => &[],
    }
  }
}

/// The plugins found in a folder. Features ask it for a capability and offer whatever plugins give it.
#[derive(Clone, Debug, Default)]
pub struct PluginRegistry {
  plugins: Vec<Plugin>,
  problems: Vec<PluginProblem>,
  /// The ids of the plugins that can be used: every plugin they depend on is installed and can be used too.
  usable: HashSet<String>,
}

impl PluginRegistry {
  /// Reads every plugin in a folder: each folder in it that has a `plugin.yml`. The plugins are in the order of their folders' names, so
  /// the result does not depend on the order the system lists them. A plugin that cannot be used is reported in
  /// [`PluginRegistry::problems`] and the others are still read; so is a second plugin with an id that is already taken (the first wins).
  /// A folder that is not there holds no plugins.
  ///
  /// - `p_dir`: The folder that holds the plugins.
  pub fn scan(p_dir: impl AsRef<Path>) -> Self {
    let mut registry = Self::default();
    let Ok(listing) = std::fs::read_dir(p_dir.as_ref()) else {
      return registry;
    };
    let mut folders: Vec<PathBuf> = listing
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .filter(|path| path.join("plugin.yml").is_file())
      .collect();
    folders.sort();
    for dir in folders {
      match Plugin::read(&dir) {
        Ok(plugin) if registry.plugin(&plugin.manifest.id).is_some() => registry.problems.push(PluginProblem {
          dir,
          error: PluginError::invalid_manifest(format!("another plugin already has the id `{}`", plugin.manifest.id)),
        }),
        Ok(plugin) => registry.plugins.push(plugin),
        Err(error) => registry.problems.push(PluginProblem { dir, error }),
      }
    }
    // A plugin can be used once everything it depends on can. Start from the plugins that need nothing and add the plugins whose
    // dependencies are in the set, until no more can be added: a plugin in a cycle is never added.
    loop {
      let ready: Vec<String> = registry
        .plugins
        .iter()
        .filter(|plugin| !registry.usable.contains(&plugin.manifest.id))
        .filter(|plugin| plugin.manifest.dependencies.iter().all(|dependency| registry.usable.contains(&dependency.id)))
        .map(|plugin| plugin.manifest.id.clone())
        .collect();
      if ready.is_empty() {
        break;
      }
      registry.usable.extend(ready);
    }
    registry
  }

  /// Whether a plugin can be used: every plugin it depends on is installed and can be used.
  /// - `p_plugin`: A plugin of this registry.
  pub fn is_usable(&self, p_plugin: &Plugin) -> bool {
    self.usable.contains(&p_plugin.manifest.id)
  }

  /// The ids of the plugins that `p_plugin` depends on that cannot be used: not installed, or not usable themselves. Empty for a plugin
  /// that can be used. The app looks these up in its catalog to offer to install them.
  /// - `p_plugin`: A plugin of this registry.
  pub fn missing_dependencies(&self, p_plugin: &Plugin) -> Vec<String> {
    p_plugin
      .manifest
      .dependencies
      .iter()
      .filter(|dependency| !self.usable.contains(&dependency.id))
      .map(|dependency| dependency.id.clone())
      .collect()
  }

  /// The plugins that can be used.
  pub fn plugins(&self) -> &[Plugin] {
    &self.plugins
  }

  /// The plugins in the folder that could not be used.
  pub fn problems(&self) -> &[PluginProblem] {
    &self.problems
  }

  /// A plugin by its id.
  /// - `p_id`: The plugin's id.
  pub fn plugin(&self, p_id: &str) -> Option<&Plugin> {
    self.plugins.iter().find(|plugin| plugin.manifest.id == p_id)
  }

  /// Everything the plugins that can be used add, in the order of the plugins and then of the manifest: models, then pipelines, then
  /// scripts, controls, then workspaces. A plugin whose dependencies are not met adds nothing until they are.
  pub fn contributions(&self) -> Vec<Contribution<'_>> {
    self
      .plugins
      .iter()
      .filter(|plugin| self.is_usable(plugin))
      .flat_map(|plugin| {
        let manifest = &plugin.manifest;
        manifest
          .models
          .iter()
          .map(move |model| Contribution::Model(plugin, model))
          .chain(manifest.pipelines.iter().map(move |pipeline| Contribution::Pipeline(plugin, pipeline)))
          .chain(manifest.scripts.iter().map(move |script| Contribution::Script(plugin, script)))
          .chain(manifest.controls.iter().map(move |control| Contribution::Control(plugin, control)))
          .chain(manifest.workspaces.iter().map(move |workspace| Contribution::Workspace(plugin, workspace)))
      })
      .collect()
  }

  /// Everything the plugins add that has a capability, such as `inpaint`: what a feature offers.
  /// - `p_capability`: The capability to ask for.
  pub fn with_capability(&self, p_capability: &str) -> Vec<Contribution<'_>> {
    self
      .contributions()
      .into_iter()
      .filter(|contribution| contribution.capabilities().iter().any(|capability| capability == p_capability))
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A folder of plugins in the system's temporary folder that is removed when the test ends.
  struct Folder(PathBuf);

  impl Folder {
    fn new(p_name: &str) -> Self {
      let path = std::env::temp_dir().join(format!("abra-plugins-{p_name}-{}", std::process::id()));
      let _ = std::fs::remove_dir_all(&path);
      std::fs::create_dir_all(&path).unwrap();
      Self(path)
    }

    fn plugin(&self, p_name: &str, p_manifest: &str, p_files: &[&str]) {
      let dir = self.0.join(p_name);
      std::fs::create_dir_all(&dir).unwrap();
      std::fs::write(dir.join("plugin.yml"), p_manifest).unwrap();
      for file in p_files {
        std::fs::write(dir.join(file), b"x").unwrap();
      }
    }
  }

  impl Drop for Folder {
    fn drop(&mut self) {
      let _ = std::fs::remove_dir_all(&self.0);
    }
  }

  fn model_plugin(p_id: &str, p_capability: &str) -> String {
    format!(
      "id: test.{p_id}\nname: {p_id}\nversion: 1\ncontributes:\n  models:\n    - id: {p_id}-model\n      title: Model\n      capabilities: [{p_capability}]\n      model: m.onnx\n"
    )
  }

  #[test]
  fn a_feature_asks_for_a_capability_and_gets_what_the_plugins_give() {
    let folder = Folder::new("capability");
    folder.plugin("b-large", &model_plugin("lama", "inpaint"), &["m.onnx", "m.yml"]);
    folder.plugin("a-fast", &model_plugin("migan", "inpaint"), &["m.onnx", "m.yml"]);
    folder.plugin("c-other", &model_plugin("segmenter", "mask"), &["m.onnx", "m.yml"]);
    let registry = PluginRegistry::scan(&folder.0);
    assert!(registry.problems().is_empty(), "{:?}", registry.problems());
    let found: Vec<_> = registry
      .with_capability("inpaint")
      .iter()
      .map(|contribution| contribution.plugin().manifest.id.clone())
      .collect();
    assert_eq!(found, ["test.migan", "test.lama"], "in the order of the folders' names");
    assert!(registry.with_capability("nothing").is_empty());
    let one = &registry.with_capability("mask")[0];
    assert_eq!((one.id(), one.title()), ("segmenter-model", "Model"));
    assert!(one.plugin().path("m.onnx").is_file());
  }

  #[test]
  fn a_plugin_that_cannot_be_used_does_not_take_the_others_away() {
    let folder = Folder::new("problems");
    folder.plugin("good", &model_plugin("good", "inpaint"), &["m.onnx", "m.yml"]);
    folder.plugin("no-file", &model_plugin("no-file", "inpaint"), &["m.onnx"]);
    folder.plugin("bad-manifest", "id: [", &[]);
    folder.plugin("same-id", &model_plugin("good", "inpaint"), &["m.onnx", "m.yml"]);
    std::fs::create_dir_all(folder.0.join("not-a-plugin")).unwrap();
    let registry = PluginRegistry::scan(&folder.0);
    assert_eq!(registry.plugins().len(), 1);
    assert_eq!(registry.plugins()[0].manifest.id, "test.good");
    let reasons: Vec<String> = registry.problems().iter().map(|problem| problem.error.to_string()).collect();
    assert_eq!(reasons.len(), 3, "{reasons:?}");
    assert!(reasons.iter().any(|reason| reason.contains("m.yml")), "a file the plugin names is missing");
    assert!(reasons.iter().any(|reason| reason.contains("not valid YAML")));
    assert!(reasons.iter().any(|reason| reason.contains("already has the id")));
  }

  /// A plugin that adds a script with a capability, and depends on the plugins given.
  fn script_plugin(p_id: &str, p_depends_on: &[&str]) -> String {
    let dependencies: String = p_depends_on.iter().map(|id| format!("  - id: test.{id}\n")).collect();
    let section = if dependencies.is_empty() { String::new() } else { format!("dependencies:\n{dependencies}") };
    format!(
      "id: test.{p_id}\nname: {p_id}\nversion: 1\n{section}contributes:\n  scripts:\n    - id: run\n      title: Run\n      capabilities: [{p_id}-run]\n      entry: s.js\n"
    )
  }

  #[test]
  fn a_plugin_is_used_only_when_the_plugins_it_depends_on_are_installed() {
    let folder = Folder::new("dependencies");
    folder.plugin("beauty", &script_plugin("beauty", &["models"]), &["s.js"]);
    let registry = PluginRegistry::scan(&folder.0);
    let beauty = registry.plugin("test.beauty").expect("it is listed, so the app can say what it needs");
    assert!(!registry.is_usable(beauty));
    assert_eq!(registry.missing_dependencies(beauty), ["test.models"]);
    assert!(registry.with_capability("beauty-run").is_empty(), "it adds nothing until its dependencies are met");

    // The plugin it needs is installed: now it is used, and nothing is missing.
    folder.plugin("models", &script_plugin("models", &[]), &["s.js"]);
    let registry = PluginRegistry::scan(&folder.0);
    let beauty = registry.plugin("test.beauty").unwrap();
    assert!(registry.is_usable(beauty) && registry.missing_dependencies(beauty).is_empty());
    assert_eq!(registry.with_capability("beauty-run").len(), 1);
  }

  #[test]
  fn a_dependency_that_cannot_be_used_itself_leaves_the_plugin_unusable_and_a_cycle_is_never_met() {
    let folder = Folder::new("chain");
    // top needs middle, middle needs bottom, and bottom is not installed.
    folder.plugin("top", &script_plugin("top", &["middle"]), &["s.js"]);
    folder.plugin("middle", &script_plugin("middle", &["bottom"]), &["s.js"]);
    // a and b need each other.
    folder.plugin("a", &script_plugin("a", &["b"]), &["s.js"]);
    folder.plugin("b", &script_plugin("b", &["a"]), &["s.js"]);
    let registry = PluginRegistry::scan(&folder.0);
    for id in ["test.top", "test.middle", "test.a", "test.b"] {
      assert!(!registry.is_usable(registry.plugin(id).unwrap()), "{id}");
    }
    assert_eq!(
      registry.missing_dependencies(registry.plugin("test.top").unwrap()),
      ["test.middle"],
      "what is missing is what cannot be used"
    );
    assert_eq!(registry.missing_dependencies(registry.plugin("test.middle").unwrap()), ["test.bottom"]);
    assert!(registry.contributions().is_empty());

    folder.plugin("bottom", &script_plugin("bottom", &[]), &["s.js"]);
    let registry = PluginRegistry::scan(&folder.0);
    assert!(registry.is_usable(registry.plugin("test.top").unwrap()), "the chain is met from the bottom up");
    assert!(!registry.is_usable(registry.plugin("test.a").unwrap()), "a cycle stays unmet");
  }

  #[test]
  fn the_controls_of_a_plugin_are_found_with_their_names_and_need_their_scripts() {
    let manifest = |entry: &str| {
      format!(
        "id: test.pack
name: Pack
version: 1
contributes:
  controls:
    - root: Beauty/Makeup
      items:
        - path: Eyes/Eyeliner
          entry: {entry}
    - root: Beauty/Hair
      items:
        - path: Color
          entry: hair.js
"
      )
    };
    let folder = Folder::new("controls");
    folder.plugin("pack", &manifest("eyeliner.js"), &["eyeliner.js", "hair.js"]);
    folder.plugin("missing", &manifest("gone.js").replace("id: test.pack", "id: test.missing"), &["hair.js"]);
    let registry = PluginRegistry::scan(&folder.0);
    assert_eq!(
      registry.problems().len(),
      1,
      "a control whose script is not there is reported: {:?}",
      registry.problems()
    );
    assert!(matches!(&registry.problems()[0].error, PluginError::FileNotFound(file) if file == "gone.js"));

    let controls: Vec<(String, String)> = registry
      .contributions()
      .into_iter()
      .filter_map(|contribution| match contribution {
        Contribution::Control(_, control) => Some((control.root.clone(), control.name().to_string())),
        _ => None,
      })
      .collect();
    assert_eq!(
      controls,
      [
        ("Beauty/Makeup".to_string(), "Eyeliner".to_string()),
        ("Beauty/Hair".to_string(), "Color".to_string())
      ]
    );
    let first = registry.contributions().into_iter().next().unwrap();
    assert_eq!((first.id(), first.title()), ("Eyes/Eyeliner", "Eyeliner"));
    assert!(first.capabilities().is_empty(), "a control is placed by its root, not found by a capability");
  }

  #[test]
  fn a_folder_that_is_not_there_holds_no_plugins() {
    let registry = PluginRegistry::scan(std::env::temp_dir().join("abra-plugins-does-not-exist"));
    assert!(registry.plugins().is_empty() && registry.problems().is_empty());
  }
}
