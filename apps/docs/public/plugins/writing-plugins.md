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
- **The script** is TypeScript in `src/` named like its entry (`src/lipstick.ts` builds to `lipstick.js`). Its default export is a function that returns the control, in the same shape the app's own controls use. It can import the Abra library (`@abra/core/...`), `@alakazam/host` (what the app offers plugins: the face masks, the layer painter and a few helpers), and other files of the plugin by relative path, which are built into the same file.
- **Types** are checked with the plugin's own `tsconfig.json`, which extends `../tsconfig.base.json` (the app's settings, plus the `@alakazam/host` module). A script that imports something the app does not offer plugins is an error.

## Build and package

For an Alakazam plugin with TypeScript controls or scripts, put its source in `apps/alakazam/mobile/plugins/<folder>/src/`. Add a `tsconfig.json` beside `plugin.yml` that extends the shared plugin config and sets an output folder:

```json
{
  "extends": "../tsconfig.base.json",
  "compilerOptions": { "outDir": "dist" },
  "include": ["src/**/*"]
}
```

Name each source file after its manifest entry: `entry: color.js` is built from `src/color.ts`. Relative TypeScript imports are bundled into that entry. Imports such as `@abra/core/color` and `@alakazam/host` stay external for the app to provide.

From the repository root, build and package one plugin:

```sh
node scripts/build-plugins.mjs my-control
node scripts/pack-plugins.mjs my-control
```

The build type-checks the plugin, writes its JavaScript bundle to `dist/`, and updates the manifest's `files:` list with the bundle's SHA-256. The pack command rebuilds app plugins being packed, checks every listed file against its hash, and assembles the upload folder at `target/plugin-packages/my-control/`. That folder contains `plugin.yml` and the downloadable files at the paths named in the manifest. It prints the object paths and the catalog entry, including the hash of `plugin.yml`.

To build all app plugins, leave off the folder name. To package all app and AI plugins, leave off the folder name from the pack command. AI plugins do not have TypeScript to build; their model files are copied from the model crate when needed. Review the generated package before uploading it.

A script runs with all of the app's rights. Only plugins from a catalog that checks their hash are installed, and the same applies to anything you add.

## Dependencies

A plugin can need other plugins. A plugin of controls that use AI models, for example, does not carry the models: it names the plugin that does.

```yaml
dependencies:
  - id: ai-face-landmarks
  - id: ai-face-parsing
```

Each dependency is a plugin's `id`. That is all a dependency says: where to get the plugin is not in the file, because the app's `catalog.yml` lists every plugin that can be installed, and the app looks a dependency up there by its id.

- **Installing** a plugin installs the plugins it depends on that are not on the device yet, and what those depend on, before it. If one of them is not in the catalog, nothing is installed and the app says which.
- **Using** a plugin needs every plugin it depends on to be installed and usable itself. Until then the plugin is listed, with the missing ones named, but it adds nothing. Plugins that depend on each other in a circle are never usable.
- A plugin cannot depend on itself or list the same dependency twice. A dependency has only an `id`, and another key is an error, so a misspelled one is not silently ignored.

## How an app uses a plugin

An app never names a plugin. A feature asks for a capability, and offers each contribution that provides it. The Remove Objects tool, for example, asks for `inpaint`, so adding another plugin with that capability adds another choice to the tool without a change to the app.

## When a plugin does not load

`PluginRegistry::scan` reports each plugin it could not use in `problems()` with a message for a person, such as the file a plugin names that is not there. The other plugins load as usual.
