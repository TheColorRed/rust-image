---
title: Writing a plugin
order: 1
outline: deep
---

# Writing a plugin

A plugin is a folder described by `plugin.yml`. Its `id` is its identity; its source folder name can be different. For the Alakazam app, TypeScript plugin sources live in `apps/alakazam/mobile/plugins/<folder>`. The build bundles their source into the files the app downloads. AI model plugins live under `packages/ai/<crate>/plugins/<folder>`.

```text
apps/alakazam/mobile/plugins/my-control/
  plugin.yml
  tsconfig.json
  src/
    control.ts

packages/ai/<crate>/plugins/ai-example/
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

| Field          | Required | Meaning                                                                                                          |
| -------------- | -------- | ---------------------------------------------------------------------------------------------------------------- |
| `id`           | yes      | Names the plugin: lowercase words of letters and digits joined by dashes. No two plugins share one.              |
| `name`         | yes      | The name to show.                                                                                                |
| `version`      | yes      | The version, such as `1.0.0`.                                                                                    |
| `description`  | no       | What the plugin does.                                                                                            |
| `contributes`  | yes      | What it adds: `models`, `pipelines`, `scripts`, `controls` or `workspaces`. At least one contribution is needed. |
| `settings`     | no       | An ordered list of setting fields. Supports `text` and single-select `option`.                                   |
| `dependencies` | no       | The plugins that must be installed for this one to be used. See [Dependencies](#dependencies).                   |

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

| Field      | Required | Meaning                                                                 |
| ---------- | -------- | ----------------------------------------------------------------------- |
| `model`    | yes      | The `.onnx` file.                                                       |
| `manifest` | no       | The model's manifest. It is the model's name with `.yml` when left out. |
| `subjects` | no       | What the model finds, such as `people`, for showing it.                 |

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

### Workspaces

A workspace opens a full-screen plugin editor. Its JavaScript entry may export any of these, and the editor shows only the parts it exports:

| Export | What it does |
|---|---|
| `inspectorGUI()` | Adds the controls of the inspector beside the preview. Without it there is no inspector and no toolbar button for it. |
| `controlsGUI()` | Adds the controls of the bar under the preview, with the same functions as `inspectorGUI`. Without it there is no bar. |
| `previewCanvas()` | Returns the canvas the preview shows, or nothing, or a promise of it. A canvas that takes a while to make is built by the library on worker threads (`canvasOfCells`, below), never with a loop of image calls on the JS thread; while the promise is pending the canvas on show stays, with a spinner over it, and only the newest call's canvas is shown. The editor shows it directly and the engine composes it on its own thread, so nothing is drawn on the JS thread. Called when the workspace opens and after each change that is not a drag. Return the same canvas again while it is still right, and change its layers in place (in a slider's `onChange`) to move something. |
| `dispose()` | Called when the workspace closes, so the plugin can free what it made, such as the canvas. |

To build a canvas of pictures, `canvasOfCells(name, width, height, cells)` makes a canvas with a child canvas for each cell, in order, each holding its picture file scaled to cover the cell (the part that does not fit is cropped). The library reads and scales the pictures on worker threads, in parallel, and reads a file that several cells use once. It returns a promise, and the child canvases are the canvas's `canvases()`, which a slider can turn, scale and move with `setRotation`, `setScale` and `setPosition`. A cell is `{ path, x, y, width, height }` in pixels of the canvas.

The plugin owns the canvas it returns: free it (with `release`) when it returns another one, and in `dispose`. To add the native image picker to the inspector:

```ts
import { inspector } from '@alakazam/plugin';

export function inspectorGUI() {
  inspector.filePicker('Photos', {
    multiple: true,
    thumbs: true,
    onPick(files) {
      // `files` always has an array of copied local file URIs.
    },
  });
}
```

`multiple` defaults to `false`. `thumbs` defaults to `false`; when `true` the inspector shows a thumbnail of each image the user picked below the control; tapping one opens it in a full-screen viewer that swipes through all of them, and pressing and holding one starts selecting images to remove. The callback always receives an array of copied local files: the full list of images, called again with the images that are left when the user removes some, so keep the list it gives you. Each call to `inspectorGUI` or `controlsGUI` replaces the controls it made before, and the editor calls them again after a change that is not a drag.

To add a heading, like an `<h1>` to `<h6>`: `inspector.header('Options')` is an `h1`, and `inspector.header('Options', { level: 2 })` is an `h2`. The `level` option is 1 to 6 and defaults to 1.

To add two number fields side by side, such as a size or a point:

```ts
inspector.size('Image Size', {
  value: [1024, 1024],
  labels: ['Width', 'Height'],
  min: 1,
  onChange(value) {
    // `value` is `[width, height]`. The editor runs `previewCanvas` again after this.
  },
});
```

`value` defaults to `[0, 0]` and `labels` to `['Width', 'Height']`. `reset` (default `true`) shows a round reset button beside the name, which puts the numbers back to what they were when the editor opened. `min` and `max` are optional and limit both numbers. `onChange` is called with both numbers each time either field holds a number; a field that is empty or not a number changes nothing. Because a size changes what the plugin makes and is not a drag, the editor runs the plugin's controls and `previewCanvas` again after each change.

To let the person pick one of a few choices, as a bar of buttons across the width, and to add a single number:

```ts
inspector.select('Style', {
  options: ['Grid', 'Layered Grid', 'Random'],
  value: 'Grid',
  onChange(choice) {
    // `choice` is the button that was pressed.
  },
});

inspector.number('Images', {
  value: 6,
  min: 1,
  max: 100,
  onChange(count) {
    // `count` is held to `min` and `max`.
  },
});
```

`select` takes `options` (the choices) and `value` (the one selected, the first by default). `number` takes `value` (`min`, or 0, by default), `min`, `max`, `step` (1 by default, which is also how many decimals it shows) and `reset` (true by default). Both change what the plugin makes and are not drags, so the editor runs the plugin's controls and `previewCanvas` again after each change, the way it does for `size`.

To add an on/off switch:

```ts
inspector.toggle('Keep Aspect Ratio', {
  value: true,
  onChange(on) {
    // `on` is whether the switch is now on.
  },
});
```

`toggle` takes `value` (false by default) and `onChange`. Like `select` and `number` it changes what the plugin makes and is not a drag, so the editor runs the plugin's controls and `previewCanvas` again after each change.

To add a slider:

```ts
inspector.slider('Rotation', {
  min: -360,
  max: 360,
  step: 1,
  range: true,
  onChange(value) {
    // `value` is `[low, high]` here, because `range` is `true`.
  },
});
```

`max` is required. `min` defaults to `0`, `step` to `1` and `range` to `false`. `multiline` puts the name on a line of its own with the slider under it across the whole width; otherwise the name is on the left and the slider on the right. It defaults to `false`, and to `true` when `range` is `true`, since two thumbs need the width. A number field is at the end of the slider (at both ends for a range) so a value can be typed. `reset` (default `true`) shows a round reset button beside the name, which puts the slider back to where it was when the editor opened. With `range: false` the slider has one thumb and `onChange` gets a number; with `range: true` it has two thumbs and `onChange` gets `[low, high]`. `value` is where it starts: `min` by default (`[min, max]` for a range). `onChange` hears every move of a drag, and the preview is drawn again once a frame; `previewCanvas` is not called again, so change the layers of the canvas it returned.

Declare the workspace entry in `plugin.yml`:

```yaml
workspaces:
  - id: collage
    title: Collage
    entry: collage.js
```

### Controls

A control for the editor: a slider, with the code that paints it. The plugin says where its controls go and what they are called, and a small script makes each one.

```yaml
contributes:
  controls:
    - root: Beauty/Makeup # the tab, then the groups inside it, by the names people see
      items:
        - path: Lipstick # the control's name, under the root
          entry: lipstick.js
        - path: Eyes/Eyeliner # a group Eyes is made in Makeup, and Eyeliner goes in it
          entry: eyeliner.js
    - root: Beauty/Hair # a plugin can have more than one root
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
