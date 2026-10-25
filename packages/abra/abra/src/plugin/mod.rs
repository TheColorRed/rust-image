//! Runtime plugins: things an app gains by adding a folder, with no rebuild.
//!
//! A plugin is a folder with a `plugin.yml` that says what it adds. The folder's name does not matter to the plugin, wherever it is kept
//! (on a device, or where it is downloaded from): the plugin is named by its `id`. By convention the folders of plugins that add AI models
//! start with `ai-`, so they can be told apart. Nothing is compiled into the app: the app scans a folder, and a plugin
//! that is there is available. What a plugin adds are *contributions*, each with the capabilities it provides, and a feature asks for a
//! capability and offers whatever plugins give it, so a new plugin changes no code in the app.
//!
//! | Contribution | What it is | Run by |
//! |---|---|---|
//! | `models` | an AI model and its manifest (see `abra-ai-core`, `graph`) | the AI runtime (`AiModel`) |
//! | `pipelines` | a list of library operations with their settings | the host, which maps each `op` to the library |
//! | `scripts` | a TypeScript module that uses the same bindings as the app | the host's JavaScript engine |
//! | `controls` | a control for the editor (a slider with the code that paints it), placed by its `root` and its own `path` | the editor |
//!
//! A plugin may also declare an ordered settings form with `text` and single-select `option` fields.
//!
//! ```yaml
//! id: alakazam.lama-object-removal # author and plugin name, separated by a dot
//! name: LaMa object removal
//! version: 1.0.0
//! description: Removes objects from a photo with LaMa.
//! settings:                         # optional settings, shown in this order
//!   - type: text
//!     id: api-token
//!     attributes:
//!       label: API token
//!       placeholder: Enter a token
//!   - type: option
//!     id: quality
//!     attributes:
//!       label: Quality
//!       options: [fast, balanced, best]
//!       default: balanced
//! dependencies:                   # optional: plugins that have to be installed for this one to be used
//!   - id: alakazam.face-landmarks
//! contributes:
//!   models:
//!     - id: object-removal-large   # unique in the plugin
//!       title: Object removal (best quality)
//!       description: Larger and slower, cleaner on big areas.
//!       capabilities: [inpaint]    # what a feature can ask for
//!       subjects: [objects]        # what the model finds, for showing it
//!       model: lama_fp32.onnx      # files are relative to the plugin's folder
//!       manifest: lama_fp32.yml    # optional; always the model's name with .yml, because it is read from beside the model
//!   pipelines:
//!     - id: tilt-shift
//!       title: Tilt Shift
//!       capabilities: [effect]
//!       steps:
//!         - { op: gaussian_blur, radius: 12 }
//!   scripts:
//!     - id: collage
//!       title: Collage
//!       capabilities: [generator]
//!       entry: collage.js
//!   controls:                      # controls for the editor, under one root or several
//!     - root: Beauty/Makeup        # the tab and groups they go in, by the names people see; made if they are not there
//!       items:
//!         - path: Lipstick         # under the root; `Eyes/Eyeliner` would put it in a group Eyes
//!           entry: lipstick.js     # its default export is a function that returns the control
//! files:                           # only for a plugin that is downloaded: what to fetch, beside the plugin.yml
//!   - path: lama_fp32.onnx
//!     sha256: 1faef5301d78db7dda502fe59966957ec4b79dd64e16f03ed96913c7a4eb68d6
//!   - path: lama_fp32.yml
//!     sha256: 99efecb3535608065fcc299b33731d60be3d5f7cd7f4fb95475ce804d2dae95b
//! ```
//!
//! A plugin can depend on other plugins, by id, so it need not carry what they already have. The registry offers the contributions of a
//! plugin only when every plugin it depends on is installed and can itself be used, so a cycle is never satisfied. A plugin whose
//! dependencies are not met is still listed, with [`PluginRegistry::missing_dependencies`] saying which are missing, so the app can offer
//! to install them. Finding them is not the registry's job: the app's catalog lists the plugins that can be installed.
//!
//! [`PluginRegistry::scan`] reads every plugin in a folder. A plugin that is broken (a bad manifest, a missing file, a name already taken)
//! is reported in [`PluginRegistry::problems`] and does not stop the others, so one bad download never takes the rest away.

mod error;
mod manifest;
mod registry;

pub use error::PluginError;
pub use manifest::{
  ControlContribution, Dependency, ModelContribution, PipelineContribution, PipelineStep, PluginFile, PluginManifest,
  PluginSetting, ScriptContribution, StepValue,
};
pub use registry::{Contribution, Plugin, PluginProblem, PluginRegistry};
