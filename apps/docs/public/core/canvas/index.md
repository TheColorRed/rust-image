---
title: Canvas Overview
order: 0
outline: deep
---

# Canvas Overview

Canvas documentation covers composition, layers, positioning, transforms, sizing, resolution, and render-time effects.

A `Canvas` groups image layers and child canvases into a composition that can be flattened or saved as an image.

## Canvas guides

| Guide                                        | Covers                                                                                              |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [Canvas](./canvas)                           | Create canvases, add layers, configure resolution, nest compositions, transform, flatten, and save. |
| [Layer](./layer)                             | Visibility, opacity, blending, ordering, image access, duplication, and effects.                    |
| [Layer options](./options)                   | Size strategies, anchors, opacity, and blend mode configuration.                                    |
| [Child canvas options](./add-canvas-options) | Anchor, offset, and rotation options for nested canvases.                                           |
| [Blend modes](./blend-modes)                 | Combine layer colors with normal, tonal, contrast, and component modes.                             |
| [Positioning](./positioning)                 | Anchors, origins, absolute positions, relative positions, and child canvases.                       |
| [Transforms](./transforms)                   | Resize, crop, rotate, flip, and canvas versus layer transforms.                                     |
| [Effects](./effects)                         | Strokes, drop shadows, effect order, and canvas-level effects.                                      |

## Typical workflow

```rust
use abra::canvas::prelude::*;

let canvas = Canvas::new_blank("Composition", 1200, 800);
canvas.add_layer_from_path("Background", "assets/background.jpg", None);
canvas.add_layer_from_path("Foreground", "assets/foreground.png", None);
canvas.save("out/composition.png", None);
```
