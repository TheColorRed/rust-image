---
title: Canvas
outline: deep
order: 1
---

# Canvas

A `Canvas` is a composition of image layers and child canvases. It provides a project-sized workspace for loading images, arranging content, applying transforms, and exporting a flattened result.

## Quick start

Create a canvas, add layers, and save the composition:

```rust
use abra::canvas::prelude::*;

let canvas = Canvas::new("My Project");
canvas.add_layer_from_path("Background", "assets/background.png", None);
canvas.add_layer_from_path("Logo", "assets/logo.png", None);

canvas.save("out/result.png", None);
```

When a canvas starts at `0 x 0`, its first layer establishes the canvas dimensions. Use `Canvas::new_blank` when the output size should be explicit.

## Creating canvases

```rust
use abra::canvas::prelude::*;

let empty = Canvas::new("Project");
let blank = Canvas::new_blank("Poster", 1920, 1080);
let from_image = Canvas::new_from_path("Photo", "assets/photo.jpg", None);
```

`new` creates an empty canvas. `new_blank` creates a fixed-size blank canvas. `new_from_path` loads an image as the initial layer and sizes the canvas from it.

## Adding layers

Load a layer from a path or add an existing image:

```rust
use abra::canvas::prelude::*;
use abra::abra_core::Image;

let canvas = Canvas::new("Layers");
let background = canvas.add_layer_from_path("Background", "assets/bg.jpg", None);
let overlay = Image::read("assets/overlay.png")?;
let foreground = canvas.add_layer_from_image("Overlay", overlay, None);
```

Both methods return the created [`Layer`](./layer), so it can be configured immediately. Pass `NewLayerOptions` to control size, anchor, opacity, and blend mode while adding it. See [Layer options](./options).

## Looking up and ordering layers

Retrieve layers by index, ID, or name, and inspect the stack:

```rust
let first = canvas.get_layer_by_index(0);
let logo = canvas.get_layer_by_name("Logo");
let count = canvas.layer_count();
let layers = canvas.layers();
```

Use the returned layer to change visibility, position, stacking order, or effects. See [Layer](./layer).

## Exporting and flattening

`save` recomposes the canvas and writes the final image. `as_image` returns a flattened `Image` without changing the layer structure:

```rust
let preview = canvas.as_image();
preview.write("out/preview.png", None)?;
canvas.save("out/final.png", None);
```

Use `flatten` when the layers should be merged into one layer in the canvas:

```rust
canvas.flatten();
canvas.save("out/flattened.png", None);
```

## Nesting canvases

A canvas can contain another canvas. This is useful for grouping a composition and transforming it as one unit:

```rust
use abra::canvas::{AddCanvasOptions, Canvas};

let parent = Canvas::new_blank("Parent", 1024, 768);
let child = Canvas::new_from_path("Child", "assets/photo.jpg", None);

parent.add_canvas(child, Some(AddCanvasOptions::new().with_rotation(15.0)));
parent.save("out/nested.png", None);
```

Child canvas placement can also be controlled with anchors and canvas positioning.

## Resolution and DPI

Every canvas has a `Resolution` value. It controls physical-size metadata and the DPI used when a point-sized `Text` layer is rasterized; it does not resize existing pixel dimensions.

```rust
use abra::abra_core::Resolution;
use abra::canvas::prelude::*;

let root = Canvas::new_blank("Print composition", 2400, 1600);
root.set_resolution(Resolution::PRINT);
```

Use `Canvas::new_from_unit` when the intended physical or relative dimensions are known. Pixel dimensions continue to use `Canvas::new_blank`.

```rust
use abra::abra_core::Resolution;
use abra::canvas::prelude::{Canvas, CanvasUnit};

let poster = Canvas::new_from_unit(
  "Poster",
  CanvasUnit::Inches(27, 40, Resolution::PRINT),
);

assert_eq!(poster.dimensions::<u32>(), (8100, 12000));
```

`CanvasUnit` supports `Inches`, `Centimeters`, `Millimeters`, `Points`, and `Picas`. `Percent` requires a reference pixel area, and `Columns` requires the configured width and height of one grid cell:

```rust
let half_size = Canvas::new_from_unit(
  "Half size",
  CanvasUnit::Percent(50, 50, 1920, 1080, Resolution::SCREEN),
);

let grid = Canvas::new_from_unit(
  "Grid",
  CanvasUnit::Columns(12, 8, 80, 60, Resolution::SCREEN),
);
```

`set_resolution` changes DPI metadata without resizing pixels. Use `resample` when the composition, including child canvases and layers, must be resized to a new `CanvasUnit`:

```rust
canvas.resample(CanvasUnit::Inches(27, 40, Resolution::PRINT));
```

`resample` reuses the canvas resize pipeline to scale the full composition tree, then applies the unit's resolution metadata.

Children inherit the root resolution when attached, and later root changes propagate recursively to children and their layers. A child that calls `set_resolution` directly becomes an override and keeps that value when the root changes:

```rust
let child = Canvas::new_blank("Screen overlay", 1200, 800);
child.set_resolution(Resolution::SCREEN);
root.add_canvas(child, None);
```

Text added with `add_layer_from_image` automatically uses its canvas DPI. Point-sized text is retained and re-rasterized when inherited resolution changes:

```rust
use abra::typography::prelude::TextSize;

canvas.add_layer_from_image(
  "Title",
  font.text("Print title").with_size(TextSize::points(18)),
  None,
);
```

## Canvas transforms and positioning

A canvas can be positioned and rotated relative to its parent:

```rust
canvas.set_position(100, 50);
canvas.set_rotation(Some(15.0));

let (x, y) = canvas.position();
let rotation = canvas.rotation();
```

For image-size transforms such as resize and crop, use `canvas.transform()`:

```rust
let mut transform = canvas.transform();
transform.resize(800, 600, None);
transform.crop(0, 0, 800, 560);
```

See [Positioning](./positioning) and [Transforms](./transforms) for the complete APIs.

## Canvas effects

Apply `LayerEffects` to the entire canvas with `set_effects`:

```rust
use abra::canvas::effects::{DropShadow, LayerEffects};

canvas.set_effects(
  LayerEffects::new()
    .with_drop_shadow(DropShadow::new().with_distance(8.0).with_size(6.0)),
);
```

See [Effects](./effects).

## API summary

| API                                            | Purpose                                                        |
| ---------------------------------------------- | -------------------------------------------------------------- |
| `Canvas::new(name)`                            | Create an empty canvas.                                        |
| `Canvas::new_blank(name, width, height)`       | Create a fixed-size blank canvas.                              |
| `Canvas::new_from_unit(...)`                   | Create a canvas from `CanvasUnit` dimensions and a resolution. |
| `resample(CanvasUnit)`                         | Resize the full composition tree and apply a resolution.       |
| `Canvas::new_from_path(name, path, options)`   | Create a canvas from an image path.                            |
| `add_layer_from_path` / `add_layer_from_image` | Add image layers.                                              |
| `add_canvas`                                   | Add a child canvas.                                            |
| `resolution` / `set_resolution`                | Read or set document DPI metadata and inheritance.             |
| `get_layer_by_index` / `get_layer_by_name`     | Find layers.                                                   |
| `as_image`                                     | Return a flattened image without changing the layer structure. |
| `flatten`                                      | Merge layers into one layer.                                   |
| `save`                                         | Recompose and write the canvas.                                |
| `transform`                                    | Access canvas resize and crop operations.                      |
