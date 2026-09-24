---
title: Getting started Overview
order: 0
outline: deep
---

# Getting started Overview

Abra is a Rust image library for loading, creating, transforming, drawing, compositing, and exporting images. Start with a single `Image`, then move to `Canvas` when you need layers and composition.

## The shortest path

1. Add the `abra` crate to a Rust project.
2. Load or create an `Image`.
3. Apply an operation such as resize, crop, or fill.
4. Save the result.

```rust
use abra::prelude::*;
use abra::abra_core::ResizeTarget;

fn main() {
  let mut image = Image::read("assets/input.jpg")?;
  image.resize(ResizeTarget::FitWidth(1200), None);
  image.write("out/resized.png", None)?;
}
```

## Guides in this section

| Guide                                  | Covers                                                    |
| -------------------------------------- | --------------------------------------------------------- |
| [Install Abra](./install)              | Add the crate and understand the workspace layout.        |
| [First image](./first-image)           | Load, create, inspect, transform, and save an image.      |
| [Canvas workflow](./canvas-workflow)   | Compose images with layers, opacity, and export.          |
| [Examples and development](./examples) | Run repository examples and build the documentation site. |

## Where to go next

- Use [Color](../core/color/color) and [Fill](../core/color/fill) for drawing content.
- Use [Geometry](../core/geometry/) for paths, areas, shapes, and viewport mapping.
- Use [Canvas](../core/canvas/canvas) for the full composition API.
- Explore [Plugins](../plugins/) for higher-level workflows such as collage generation.
