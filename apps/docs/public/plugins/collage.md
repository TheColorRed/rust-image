---
title: Collage
order: 2
outline: deep
---

# Collage

`CollagePlugin` creates a canvas from multiple source images. It supports regular grids, overlapping layered grids, and random placement.

The plugin is provided by the `abra_collage` crate:

```rust
use abra::prelude::*;
use abra_collage::prelude::*;
```

## Basic collage

Load images with `ImageLoader`, create the plugin with an output size, apply it, and save the returned canvas:

```rust
let images = ImageLoader::FromPaths(vec![
  "assets/one.jpg",
  "assets/two.jpg",
  "assets/three.jpg",
  "assets/four.jpg",
]).load(LoadMode::Parallel { threads: 4 });

let mut plugin = CollagePlugin::new((1200, 800), images);
let mut result = plugin.apply().unwrap();

if let Some(canvas) = result.take_canvas_at(0) {
  canvas.save("out/collage.png", None);
}
```

`CollagePlugin::new` defaults to `CollageStyle::Grid(2, 2)`. Images are selected randomly without repetition until all provided images have been used; if the layout has more cells than images, selection starts over.

## Layout styles

### Grid

`Grid(columns, rows)` places images into evenly sized cells. Images use a cover resize strategy, so an image may be cropped to fill its cell:

```rust
let plugin = CollagePlugin::new((1200, 800), images)
  .with_style(CollageStyle::Grid(3, 2));
```

### Layered grid

`LayeredGrid(columns, rows)` places image canvases in a grid with overlap-friendly transforms. Rotation and scale ranges are useful with this style:

```rust
let plugin = CollagePlugin::new((1200, 800), images)
  .with_style(CollageStyle::LayeredGrid(3, 2))
  .with_options(
    CollageOptions::new()
      .with_rotation_range(-12.0, 12.0)
      .with_scale_range(0.9, 1.15),
  );
```

### Random

`Random(count)` places the requested number of images at random positions within the collage bounds:

```rust
let plugin = CollagePlugin::new((1200, 800), images)
  .with_style(CollageStyle::Random(12));
```

The count is clamped to at least one image. Rotation and scale are selected independently for every item when ranges are configured.

## Collage options

`CollageOptions::new()` uses these defaults:

- rotation range: `0.0..=0.0`
- scale range: `1.0..=1.0`
- transparent background
- no per-image effects

### Rotation

Positive angles rotate clockwise and negative angles rotate counter-clockwise:

```rust
let options = CollageOptions::new()
  .with_rotation_range(-15.0, 15.0);
```

The plugin samples a value from the inclusive range for each selected image where the layout supports rotation.

### Scale

Values below `1.0` shrink images; values above `1.0` enlarge them:

```rust
let options = CollageOptions::new()
  .with_scale_range(0.8, 1.25);
```

The random collage applies the range to each image. The layered grid uses it to vary cell canvas sizes.

### Background

The background accepts any value convertible to `Fill`: a color, gradient, or image:

```rust
use abra::abra_core::{Color, Fill, Gradient};

let solid = CollageOptions::new()
  .with_background(Color::black());

let gradient = CollageOptions::new()
  .with_background(Fill::Gradient(Gradient::rainbow().into()));
```

An image background is scaled with `LayerSize::Cover` to fill the collage dimensions. If no background is set, the collage remains transparent.

### Effects

Apply `LayerEffects` to the generated image canvases:

```rust
use abra::canvas::effects::{DropShadow, LayerEffects, Stroke};
use abra::abra_core::{Color, Fill};

let effects = LayerEffects::new()
  .with_stroke(
    Stroke::new()
      .with_fill(Fill::Solid(Color::white().into()))
      .with_size(8),
  )
  .with_drop_shadow(
    DropShadow::new()
      .with_distance(16.0)
      .with_size(24.0),
  );

let plugin = CollagePlugin::new((1200, 800), images)
  .with_options(CollageOptions::new().with_effects(effects));
```

## Complete example

```rust
use abra::canvas::prelude::*;
use abra::prelude::*;
use abra_collage::prelude::*;

pub fn main() {
  let images = ImageLoader::FromGlob(vec!["assets/photos/*.jpg"]).load(LoadMode::Parallel { threads: 4 });

  let mut plugin = CollagePlugin::new((1600, 1000), images)
    .with_style(CollageStyle::LayeredGrid(4, 2))
    .with_options(
      CollageOptions::new()
        .with_rotation_range(-10.0, 10.0)
        .with_scale_range(0.95, 1.1)
        .with_background(Color::from_hex(0x20252B)),
    );

  let mut result = plugin.apply().unwrap();
  let canvas = result.take_canvas_at(0).unwrap();
  canvas.save("out/collage.png", None);
}
```

## API summary

| API                                        | Purpose                                             |
| ------------------------------------------ | --------------------------------------------------- |
| `CollagePlugin::new(size, images)`         | Create a collage generator.                         |
| `with_style(CollageStyle)`                 | Select the layout.                                  |
| `with_options(CollageOptions)`             | Configure rotation, scale, background, and effects. |
| `CollageStyle::Grid(columns, rows)`        | Create an evenly partitioned grid.                  |
| `CollageStyle::LayeredGrid(columns, rows)` | Create a grid of overlapping image canvases.        |
| `CollageStyle::Random(count)`              | Place a number of images randomly.                  |
| `CollageOptions::new()`                    | Create default collage options.                     |
| `with_rotation_range(min, max)`            | Set random rotation bounds.                         |
| `with_scale_range(min, max)`               | Set random scale bounds.                            |
| `with_background(fill)`                    | Set a solid, gradient, or image background.         |
| `with_effects(effects)`                    | Apply effects to generated image canvases.          |
