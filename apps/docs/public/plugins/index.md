---
title: Plugins Overview
order: 0
outline: deep
---

# Plugins Overview

A plugin is something an app gains by adding a folder. Nothing is compiled into the app: the app scans a folder of plugins when it starts, and a plugin that is there just works. The app's code is not changed to add one.

A plugin is a folder with a `plugin.yml` that says what it adds. What it adds are _contributions_, and each contribution lists the _capabilities_ it provides. A feature of the app asks for a capability (for example `inpaint`, to fill a masked part of a photo) and offers whatever plugins provide it, so a new plugin shows up in the features that can use it.

## What a plugin can add

| Contribution | What it is                                                             | Run by                                                  |
| ------------ | ---------------------------------------------------------------------- | ------------------------------------------------------- |
| `models`     | An AI model and its manifest.                                          | The AI runtime (`AiModel`), from the manifest alone.    |
| `pipelines`  | A list of library operations, each with its settings.                  | The host, which maps each `op` to the library.          |
| `scripts`    | A TypeScript module that uses the same bindings as the app itself.     | The host's JavaScript engine.                           |
| `controls`   | A control for the editor, placed in a tab and group the plugin names.  | The editor, which runs the control's script.            |

A plugin can also declare an optional settings form with `text` and single-select `option` fields. See [Writing a plugin](./writing-plugins#settings).

## A plugin

```yaml
id: lama-object-removal
name: LaMa object removal
version: 1.0.0
description: Removes objects from a photo with LaMa.
contributes:
  models:
    - id: object-removal-large
      title: Object removal (best quality)
      capabilities: [inpaint]
      subjects: [objects]
      model: lama_fp32.onnx
      manifest: lama_fp32.yml
```

See [Writing a plugin](./writing-plugins) for every field, and for how a model's manifest says how it is fed.

A plugin can depend on other plugins by id, so it does not carry what another already has. The app finds and installs them from its catalog. See [Writing a plugin](./writing-plugins#dependencies).

## Finding plugins

`PluginRegistry::scan(folder)` reads every plugin in a folder. A plugin that cannot be used (a bad manifest, a file it names that is missing, an id that is already taken) is listed in `problems()` and does not stop the others.

```rust
use abra::plugin::PluginRegistry;

let registry = PluginRegistry::scan("plugins");

for problem in registry.problems() {
  eprintln!("{}: {}", problem.dir.display(), problem.error);
}

// What a feature offers: everything that can fill a masked part of a photo.
for contribution in registry.with_capability("inpaint") {
  println!("{} (from {})", contribution.title(), contribution.plugin().manifest.name);
}
```

The same scan is available to the apps as `scanPlugins(folder)` in the bindings.

## Included code

These are libraries, not runtime plugins. They are Rust crates that an app links in:

| Crate                | Purpose                                                             |
| -------------------- | ------------------------------------------------------------------- |
| [Collage](./collage) | Compose multiple images into grid, layered-grid, or random layouts. |
