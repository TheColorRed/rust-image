---
title: Canvas workflow
order: 4
outline: deep
---

# Canvas workflow

Use `Canvas` when an image is made from multiple layers. A canvas handles layer order, opacity, positioning, blending, transforms, effects, and final export.

## Create a canvas

Start with an empty canvas and add a background layer:

```rust
use abra::canvas::prelude::*;

let canvas = Canvas::new("Poster");
let background = canvas.add_layer_from_path("Background", "assets/background.jpg", None);
```

Use `Canvas::new_blank` when the output dimensions should be fixed before adding content:

```rust
let canvas = Canvas::new_blank("Poster", 1200, 800);
```

## Add and configure layers

`add_layer_from_path` and `add_layer_from_image` return the created layer:

```rust
use abra::canvas::{Anchor, LayerSize, NewLayerOptions};

let options = NewLayerOptions::new()
  .with_size(LayerSize::Contain(None))
  .with_anchor(Anchor::Center)
  .with_opacity(0.9);

let photo = canvas.add_layer_from_path("Photo", "assets/photo.jpg", Some(options));
let logo = canvas.add_layer_from_path("Logo", "assets/logo.png", None);

logo.anchor_to_canvas(Anchor::BottomRight);
logo.set_global_position(-24, -24);
```

## Compose multiple images

Layers are composited in stack order. Reorder and blend them as needed:

```rust
use abra::abra_core::blend;

logo.move_to(LayerMove::Top);
photo.set_opacity(0.85);
photo.set_blend_mode(blend::normal);
```

Look up layers by name when configuring a composition in separate steps:

```rust
let overlay = canvas.get_layer_by_name("Logo").unwrap();
overlay.set_visible(true);
```

## Transform a layer

Transforms mutate the layer's image and mark the parent canvas for recomposition:

```rust
use abra::abra_core::ResizeTarget;

photo.transform()
  .resize(ResizeTarget::FitWidth(900), None)
  .crop(0, 0, 900, 600)
  .rotate(-2.0, None);
```

## Add effects

Queue render-time effects on a layer:

```rust
use abra::canvas::effects::{DropShadow, LayerEffects};

logo.set_effects(
  LayerEffects::new()
    .with_drop_shadow(DropShadow::new().with_distance(10.0).with_size(8.0)),
);
```

## Export the composition

`as_image` returns a flattened image while preserving the canvas's layer structure. `flatten` permanently merges the layers:

```rust
let preview = canvas.as_image();
preview.write("out/preview.png", None)?;

canvas.save("out/poster.png", None);
```

For a single-layer output canvas:

```rust
canvas.flatten();
canvas.save("out/poster-flattened.png", None);
```

## Complete example

```rust
use abra::canvas::prelude::*;
use abra::prelude::*;

fn main() {
  let canvas = Canvas::new_blank("Poster", 1200, 800);
  canvas.add_layer_from_path("Background", "assets/background.jpg", None);

  let logo = canvas.add_layer_from_path(
    "Logo",
    "assets/logo.png",
    Some(
      NewLayerOptions::new()
        .with_anchor(Anchor::BottomRight)
        .with_opacity(0.9),
    ),
  );
  logo.set_global_position(-24, -24);

  canvas.save("out/poster.png", None);
}
```

See the [Canvas](../core/canvas/canvas), [Layer](../core/canvas/layer), and [Layer options](../core/canvas/options) references for the complete API.
