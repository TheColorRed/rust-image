---
title: Writing a plugin
order: 1
outline: deep
---

# Writing a plugin

A plugin is a folder described by `plugin.yml`. Its `id` is its identity; its source folder name can be different. For the Alakazam app, TypeScript plugin sources live in `apps/alakazam/mobile/plugins/<folder>`. The build bundles their source into the files the app downloads. AI model plugins live under `packages/abra/ai/<crate>/plugins/<folder>`.

```text
apps/alakazam/mobile/plugins/my-control/
  plugin.yml
  tsconfig.json
  src/
    control.ts

packages/abra/ai/<crate>/plugins/ai-example/
  plugin.yml
  example.onnx
  example.yml
```

An installed plugin has its manifest and downloaded files together in the app's plugins folder:

```text
plugins/
  ai-lama-object-removal/
    plugin.yml
    lama_fp32.onnx
    lama_fp32.yml
```

## plugin.yml

| Field         | Required | Meaning                                                                                                |
| ------------- | -------- | ------------------------------------------------------------------------------------------------------ |
| `id`          | yes      | Names the plugin: lowercase words of letters and digits joined by dashes. No two plugins share one.                                    |
| `name`        | yes      | The name to show.                                                                                      |
| `version`     | yes      | The version, such as `1.0.0`.                                                                          |
| `description` | no       | What the plugin does.                                                                                  |
| `contributes` | yes      | What it adds: `models`, `pipelines`, `scripts` or `controls`. At least one contribution is needed.    |
| `settings`    | no       | An ordered list of setting fields. Supports `text` and single-select `option`. |
| `dependencies`| no       | The plugins that must be installed for this one to be used. See [Dependencies](#dependencies).         |

### Settings

Settings are optional. When present, they are an ordered list of fields with a stable `id` and a visible `label`. `description`, `default`, and `required` are optional. Defaults and option values are text for now.

```yaml
settings:
  - type: text
    id: api-token
    attributes:
      label: API token
      description: Token used to access the service.
      placeholder: Enter a token
      value: ''
    validations:
      required: true
  - type: option
    id: quality
    attributes:
      label: Quality
      options: [fast, balanced, best]
      default: balanced
    validations:
      required: true
```

Each field has an `id`, a `type`, and `attributes` with a required `label` and optional `description`. A `text` field shows a text input and can have a `placeholder` and initial `value`. An `option` shows a single-selection list and needs a non-empty `options` list; its optional `default` must match one of those options. `validations.required` defaults to `false`.

Each model, pipeline, or script contribution has an `id` (the same rules, and unique in the plugin), a `title`, an optional `description`, and `capabilities`: a list of at least one word that a feature asks for, such as `inpaint`. A contribution with no capability could not be found. Controls instead use a root, a visible path, and a script entry, as described below.

Files are written relative to the plugin's folder, with `/` between names. A path may not start with `/`, hold `..`, or hold a drive.

### Models

An AI model that the AI runtime runs from its manifest.

| Field      | Required | Meaning                                                                     |
| ---------- | -------- | --------------------------------------------------------------------------- |
| `model`    | yes      | The `.onnx` file.                                                           |
| `manifest` | no       | The model's manifest. It is the model's name with `.yml` when left out.     |
| `subjects` | no       | What the model finds, such as `people`, for showing it.                     |

The model's manifest says how the model is fed and what it answers with. It can describe every input tensor (a picture, a mask, a constant, as numbers or bytes), run the model on a window round a mask, and paste only the masked pixels back, so most models need no code at all. See `abra_ai_core::graph` for every setting.

### Pipelines

A list of library operations run in order.

```yaml
pipelines:
  - id: soften
    title: Soften
    capabilities: [effect]
    steps:
      - { op: gaussian_blur, radius: 12 }
      - { op: saturation, amount: 0.2 }
```

Each step has an `op`, which is the library operation as the host knows it, and its settings. A setting is a number, text, `true` or `false`, or a list of numbers.

### Scripts

A TypeScript module for logic that is more than a list of steps, run by the host's JavaScript engine against the same bindings the app itself uses.

```yaml
scripts:
  - id: collage
    title: Collage
    capabilities: [generator]
    entry: collage.js
```

### Controls

A control for the editor: a slider, with the code that paints it. The plugin says where its controls go and what they are called, and a small script makes each one.

```yaml
contributes:
  controls:
    - root: Beauty/Makeup          # the tab, then the groups inside it, by the names people see
      items:
        - path: Lipstick           # the control's name, under the root
          entry: lipstick.js
        - path: Eyes/Eyeliner      # a group Eyes is made in Makeup, and Eyeliner goes in it
          entry: eyeliner.js
    - root: Beauty/Hair            # a plugin can have more than one root
      items:
        - path: Hair Color
          entry: hair-color.js
```

- **The root** is matched to the names in the app, ignoring case. A tab or group that is not there is made, so a plugin can add a tab of its own. If the plugin is taken away, a tab or group it made goes with it once it is empty.
- **The path** is the control's name, which is what people see, and so the script does not need to say it. A path with more names puts the control in groups under the root.
- **The order** is the order written here.
- **The script** is TypeScript in `src/` named like its entry (`src/lipstick.ts` builds to `lipstick.js`). Its default export is a function that returns the control, in the same shape the app's own controls use. It can import the Abra library (`@abra/core/...`), `@alakazam/plugin` (what the app offers plugins: the face masks, the layer painter and a few helpers), and other files of the plugin by relative path, which are built into the same file.
- **Types** in app-owned plugins are checked with the app's plugin config. The public npm builder does not require the Alakazam app source: it transpiles TypeScript and leaves documented runtime imports external. Plugin authors can add their own type-checking setup if they want one.

## Build and package

For a plugin with TypeScript controls or scripts, put its source in `src/` beside `plugin.yml`. The npm build tool bundles its TypeScript entries and relative imports without needing the Alakazam app source code.

Each source file matches its manifest entry: `entry: color.js` is built from `src/color.ts`. Relative imports are bundled into the entry. Imports such as `@abra/core/color` and `@alakazam/plugin` stay external for Alakazam to provide at runtime.

Build from the plugin folder, or pass the folder path:

```sh
npx @alakazam/plugin build
# or
npx @alakazam/plugin build ./my-plugin
```

Add `@alakazam/plugin` and `@abra/core` to your plugin's `devDependencies` to type-check host API imports in your own TypeScript project. The package provides the declarations for `@alakazam/plugin`; Alakazam supplies its runtime implementation when the plugin loads.

The build updates the `files:` list and SHA-256 hashes in `plugin.yml`, then writes `dist/<id>.akzx`. An `.akzx` package is a ZIP archive. Plugin authors can install it from **Plugins → Local plugin package → Choose .akzx** without uploading to S3. The local installer verifies every file against its manifest hash. Declared dependencies still need to be installed before the plugin can be used.



In this repository, `node scripts/build-plugins.mjs <folder>` delegates to the same npm builder and also type-checks app-owned plugins. `scripts/pack-plugins.mjs [folder ...]` is for maintainers preparing catalog uploads; plugin authors do not need it. It checks files and assembles upload folders under `target/plugin-packages/`, including model files from the AI crates when needed. Each upload folder contains exactly `plugin.yml` and `<id>.akzx`; the catalog entry has `uri`/`sha256` for the manifest and `archive_uri`/`archive_sha256` for the `.akzx` package. The app reads the manifest for metadata and dependencies, then installs the package after verifying both hashes and each file hash in the manifest.

A plugin script runs with all of the app's rights. Install only packages from authors you trust. File hashes detect a damaged or mismatched package; they do not identify who made it.

## Dependencies

A plugin can need other plugins. A plugin of controls that use AI models, for example, does not carry the models: it names the plugin that does.

```yaml
dependencies:
  - id: alakazam.face-landmarks
  - id: alakazam.face-parsing
```

Each plugin `id` uses `<author>.<id>` so plugins from different authors can use the same name. The app's `catalog.yml` lists every plugin that can be installed, and the app looks dependencies up there by their full IDs.

- **Installing** a plugin installs the plugins it depends on that are not on the device yet, and what those depend on, before it. If one of them is not in the catalog, nothing is installed and the app says which.
- **Using** a plugin needs every plugin it depends on to be installed and usable itself. Until then the plugin is listed, with the missing ones named, but it adds nothing. Plugins that depend on each other in a circle are never usable.
- A plugin cannot depend on itself or list the same dependency twice. A dependency has only an `id`, and another key is an error, so a misspelled one is not silently ignored.

## How an app uses a plugin

An app never names a plugin. A feature asks for a capability, and offers each contribution that provides it. The Remove Objects tool, for example, asks for `inpaint`, so adding another plugin with that capability adds another choice to the tool without a change to the app.

## When a plugin does not load

`PluginRegistry::scan` reports each plugin it could not use in `problems()` with a message for a person, such as the file a plugin names that is not there. The other plugins load as usual.
