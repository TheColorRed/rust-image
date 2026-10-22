---
title: Writing a plugin
order: 1
outline: deep
---

# Writing a plugin

A plugin is a folder. Put a `plugin.yml` in it, put the files it names beside it, and put the folder in the plugins folder of the app. The folder can be called anything, here and wherever it is downloaded from: the plugin is named by the `id` in its `plugin.yml`, not by its folder. By convention the folders of plugins that add AI models start with `ai-`, so they can be told apart.

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
| `contributes` | yes      | What it adds: `models`, `pipelines` and `scripts`. At least one contribution is needed.                |
| `dependencies`| no       | The plugins that must be installed for this one to be used. See [Dependencies](#dependencies).         |

Every contribution has an `id` (the same rules, and unique in the plugin), a `title`, an optional `description`, and `capabilities`: a list of at least one word that a feature asks for, such as `inpaint`. A contribution with no capability could not be found.

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
