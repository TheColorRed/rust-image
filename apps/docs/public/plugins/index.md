---
title: Plugins Overview
order: 0
outline: deep
---

# Plugins Overview

Abra plugins extend the library with higher-level image generation and processing workflows. A plugin implements the `Plugin` trait and returns a `PluginResult` containing canvases, images, or layers.

## Plugin lifecycle

A plugin exposes a name, a description, and an `apply` method:

```rust
use abra::plugin::{Plugin, PluginError, PluginResult};

fn run_plugin<'a>(plugin: &mut impl Plugin<'a>) -> Result<PluginResult<'a>, PluginError> {
  println!("{}: {}", plugin.name(), plugin.description());
  plugin.apply()
}
```

`apply` is the plugin's execution entry point. It may return an error when parameters or required resources are invalid.

## Plugin results

`PluginResult` can contain canvases, images, and layers:

```rust
let mut result = plugin.apply()?;

if let Some(canvas) = result.take_canvas_at(0) {
  canvas.save("out/plugin-result.png", None);
}
```

Use `canvas_at`, `image_at`, and `layer_at` to borrow outputs. Use the `take_*` methods when the output should be owned and removed from the result.

## Included plugins

| Plugin                     | Purpose                                                                     | Status    |
| -------------------------- | --------------------------------------------------------------------------- | --------- |
| [Collage](./collage)       | Compose multiple images into grid, layered-grid, or random layouts.         | Available |
| [Tilt-shift](./tilt-shift) | Intended to simulate a miniature scene with blur, saturation, and contrast. | API stub  |

## Collage workflow

The Collage plugin accepts loaded images, creates a canvas, and returns it through a `PluginResult`:

```rust
use abra::prelude::*;
use abra_collage::prelude::*;

let images = ImageLoader::FromPaths(vec![
  "assets/one.jpg",
  "assets/two.jpg",
  "assets/three.jpg",
  "assets/four.jpg",
]).load(LoadMode::Parallel { threads: 4 });

let mut collage = CollagePlugin::new((1200, 800), images)
  .with_style(CollageStyle::Grid(2, 2));

let mut result = collage.apply()?;
let canvas = result.take_canvas_at(0).unwrap();
canvas.save("out/collage.png", None);
```

See [Collage](./collage) for layout and styling options.
